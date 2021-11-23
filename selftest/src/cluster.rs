use crate::test_settings::TestSettings;
use anyhow::{format_err, Context, Result};
use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::serde::de::DeserializeOwned;
use kube::{
    api::ListParams,
    config::{KubeConfigOptions, Kubeconfig},
    Api, Client, Config,
};
use model::constants::{LABEL_COMPONENT, LABEL_PROVIDER_NAME, NAMESPACE};
use std::fmt::Debug;
use std::{convert::TryInto, fs::File};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use tokio::time::Duration;

pub const KUBECONFIG_FILENAME: &str = "kubeconfig.yaml";
pub const KUBECONFIG_INTERNAL_FILENAME: &str = "kubeconfig_internal.yaml";

/// Represents a `kind` cluster. The `Drop` trait is implemented deleting the `kind` cluster when it
/// goes out of scope.
#[derive(Debug)]
pub struct Cluster {
    name: String,
    kubeconfig_dir: TempDir,
}

impl Cluster {
    /// Creates a `Cluster` while initializing a kind cluster. If a cluster named `cluster_name`
    ///  already exists, it will be deleted.
    pub fn new(cluster_name: &str) -> Result<Cluster> {
        let kubeconfig_dir = TempDir::new()?;
        Self::delete_kind_cluster(cluster_name)?;
        Self::create_kind_cluster(
            cluster_name,
            &kubeconfig_dir.path().join(KUBECONFIG_FILENAME),
        )?;
        Ok(Self {
            name: cluster_name.into(),
            kubeconfig_dir,
        })
    }

    /// Creates a kubeconfig for use within the kind network and returns its path.
    pub fn get_internal_kubeconfig(&self) -> Result<PathBuf> {
        use std::process::Command;
        let output = Command::new(TestSettings::kind_path())
            .arg("get")
            .arg("kubeconfig")
            .arg("--internal")
            .arg("--name")
            .arg(&self.name)
            .output()?;
        if !output.status.success() {
            return Err(format_err!(
                "'kind get kubeconfig --internal' with exit status '{}'\n\n{}\n\n{}",
                output.status.code().unwrap_or(1),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            ));
        }
        let mut kubeconfig_internal = File::create(
            self.kubeconfig_dir
                .path()
                .join(KUBECONFIG_INTERNAL_FILENAME),
        )?;
        kubeconfig_internal.write_all(&output.stdout)?;
        Ok(self
            .kubeconfig_dir
            .path()
            .join(KUBECONFIG_INTERNAL_FILENAME))
    }

    /// Returns the path to the kubeconfig file in the `TempDir` created for the cluster.
    pub fn kubeconfig(&self) -> PathBuf {
        self.kubeconfig_dir.path().join(KUBECONFIG_FILENAME)
    }

    /// Uses `kind load` to load an image from the machine to the kind cluster.
    pub fn load_image_to_cluster(&self, image_name: &str) -> Result<()> {
        use std::process::Command;
        let output = Command::new(TestSettings::kind_path())
            .arg("load")
            .arg("docker-image")
            .arg(image_name)
            .arg("--name")
            .arg(&self.name)
            .output()?;
        if !output.status.success() {
            return Err(format_err!(
                "'kind load docker-image failed' with exit status '{}'\n\n{}\n\n{}",
                output.status.code().unwrap_or(1),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            ));
        }
        Ok(())
    }

    /// Create the k8s client for the cluster.
    pub async fn k8s_client(&self) -> Result<Client> {
        let kubeconfig = Kubeconfig::read_from(self.kubeconfig())?;
        let config =
            Config::from_custom_kubeconfig(kubeconfig, &KubeConfigOptions::default()).await?;
        Ok(config.try_into()?)
    }

    /// Returns `true` if the controller is in the running state.
    pub async fn is_controller_running(&self) -> Result<bool> {
        let pods = self
            .find_by_label::<Pod>(LABEL_COMPONENT, "controller")
            .await?;
        if pods.is_empty() {
            return Ok(false);
        }
        for pod in pods {
            if !is_pod_running(&pod) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Waits until the controller is running. Will timeout after `duration` if not running.
    pub async fn wait_for_controller(&self, duration: Duration) -> Result<()> {
        tokio::time::timeout(duration, self.wait_for_controller_loop())
            .await
            .context("Timeout waiting for controller to be in the 'Running' state")?
    }

    /// Waits until the test pod is running. Will timeout after `duration` if not running.
    pub async fn wait_for_test_pod(&self, test_name: &str, duration: Duration) -> Result<()> {
        tokio::time::timeout(duration, self.wait_for_test_loop(test_name))
            .await
            .context("Timeout waiting for test '{}' pod to be in the 'Running' state")?
    }

    async fn wait_for_controller_loop(&self) -> Result<()> {
        loop {
            if self.is_controller_running().await? {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(750)).await;
        }
    }

    async fn wait_for_test_loop(&self, test_name: &str) -> Result<()> {
        loop {
            if self.is_test_running(test_name).await? {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(750)).await;
        }
    }

    /// Returns `true` if the `Test` named `test_name` is in the running state.
    pub async fn is_test_running(&self, test_name: &str) -> Result<bool> {
        let pods = self.find_by_label::<Pod>("job-name", test_name).await?;
        let pod = match pods.into_iter().next() {
            None => return Ok(false),
            Some(pod) => pod,
        };
        Ok(is_pod_running(&pod))
    }

    pub async fn find_by_label<T>(&self, key: &str, val: &str) -> Result<Vec<T>>
    where
        T: kube::Resource + Clone + DeserializeOwned + Debug,
        <T as kube::Resource>::DynamicType: Default,
    {
        let client = self.k8s_client().await?;
        let api = Api::<T>::namespaced(client, NAMESPACE);
        let objects = api
            .list(&ListParams {
                label_selector: Some(format!("{}={}", key, val)),
                ..Default::default()
            })
            .await?;
        Ok(objects.items)
    }

    /// Returns `true` if the `ResourceProvider` named `provider_name` is in the running state.
    pub async fn is_provider_running(&self, provider_name: &str) -> Result<bool> {
        let client = self.k8s_client().await?;
        let pod_api = Api::<Pod>::namespaced(client, NAMESPACE);
        let pods = pod_api
            .list(&ListParams {
                label_selector: Some(format!("{}={}", LABEL_PROVIDER_NAME, provider_name)),
                ..Default::default()
            })
            .await?;
        for pod in pods {
            if pod
                .status
                .unwrap_or_default()
                .phase
                .clone()
                .unwrap_or_default()
                == "Running"
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn create_kind_cluster(name: &str, kubeconfig: &Path) -> Result<()> {
        use std::process::Command;
        let output = Command::new(TestSettings::kind_path())
            .arg("--kubeconfig")
            .arg(kubeconfig.to_str().ok_or_else(|| {
                format_err!(
                    "non utf-8 path '{}'",
                    kubeconfig.join(KUBECONFIG_FILENAME).to_string_lossy()
                )
            })?)
            .arg("create")
            .arg("cluster")
            .arg("--name")
            .arg(name)
            .output()?;
        if !output.status.success() {
            return Err(format_err!(
                "'kind create cluster failed' with exit status '{}'\n\n{}\n\n{}",
                output.status.code().unwrap_or(1),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            ));
        }
        Ok(())
    }

    fn delete_kind_cluster(name: &str) -> Result<()> {
        use std::process::Command;
        let output = Command::new(TestSettings::kind_path())
            .arg("delete")
            .arg("cluster")
            .arg("--name")
            .arg(name)
            .output()?;
        if !output.status.success() {
            return Err(format_err!(
                "'kind delete cluster' failed with exit status '{}'\n\n{}\n\n{}",
                output.status.code().unwrap_or(1),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            ));
        }
        Ok(())
    }
}

impl Drop for Cluster {
    fn drop(&mut self) {
        if let Err(e) = Self::delete_kind_cluster(&self.name) {
            eprintln!("unable to delete kind cluster '{}': {}", self.name, e)
        }
    }
}

fn is_pod_running(pod: &Pod) -> bool {
    pod.status
        .as_ref()
        .and_then(|s| s.phase.as_ref().map(|s| s == "Running"))
        .unwrap_or(false)
}
