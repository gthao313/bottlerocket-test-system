use crate::error;
use crate::sonobuoy::{
    process_sonobuoy_test_results, wait_for_sonobuoy_results, wait_for_sonobuoy_status,
};
use bottlerocket_types::agent_config::{WorkloadConfig, SONOBUOY_RESULTS_FILENAME};
use log::{info, trace};
use snafu::{ensure, ResultExt};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use test_agent::InfoClient;
use testsys_model::{SecretName, TestResults};

/// Timeout for sonobuoy status to become available (seconds)
const SONOBUOY_STATUS_TIMEOUT: u64 = 900;
const SONOBUOY_BIN_PATH: &str = "/usr/bin/sonobuoy";

/// Runs the workload conformance tests according to the provided configuration and returns a test
/// result at the end.
pub async fn run_workload<I>(
    kubeconfig_path: &str,
    workload_config: &WorkloadConfig,
    results_dir: &Path,
    info_client: &I,
    aws_secret_name: &Option<&SecretName>,
) -> Result<TestResults, error::Error>
where
    I: InfoClient,
{
    info!("Processing workload test plugins");
    let mut plugin_test_args: Vec<String> = Vec::new();
    for (id, test) in workload_config.tests.iter().enumerate() {
        info!("Initializing test {}-{}", id, test.name);
        let output = Command::new(SONOBUOY_BIN_PATH)
            .arg("gen")
            .arg("plugin")
            .arg("--name")
            .arg(test.name.clone())
            .arg("--image")
            .arg(test.image.clone())
            .output()
            .context(error::WorkloadProcessSnafu)?;
        ensure!(
            output.status.success(),
            error::WorkloadTestSnafu {
                plugin: test.name.clone()
            }
        );

        // Write out the output to a file we can reference later
        let file_name = format!("{}-plugin.yaml", test.name);
        let plugin_yaml = PathBuf::from(".").join(file_name);
        let mut f = File::create(&plugin_yaml).context(error::FileWriteSnafu {
            path: plugin_yaml.display().to_string(),
        })?;
        f.write_all(&output.stdout)
            .context(error::WorkloadProcessSnafu)?;

        // Add plugin to the arguments to be passed to sonobuoy run
        plugin_test_args.append(&mut vec![
            "--plugin".to_string(),
            plugin_yaml.display().to_string(),
        ]);
    }
    let sonobuoy_image_arg = match &workload_config.sonobuoy_image {
        Some(sonobuoy_image_arg) => {
            vec!["--sonobuoy-image", sonobuoy_image_arg]
        }
        None => {
            vec![]
        }
    };

    info!("Running workload");
    info!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    let kubeconfig_arg = vec!["--kubeconfig", kubeconfig_path];
    let output = Command::new(SONOBUOY_BIN_PATH)
        .args(kubeconfig_arg.to_owned())
        .arg("run")
        .arg("--namespace")
        .arg("testsys-workload")
        .args(plugin_test_args)
        .args(sonobuoy_image_arg)
        .output()
        .context(error::WorkloadProcessSnafu)?;

    ensure!(
        output.status.success(),
        error::WorkloadRunSnafu {
            exit_code: output.status.code().unwrap_or_default(),
            stdout: &String::from_utf8(output.stdout).unwrap_or_else(|_| "".to_string()),
            stderr: &String::from_utf8(output.stderr).unwrap_or_else(|_| "".to_string()),
        }
    );

    info!("Workload testing has started, waiting for status to be available");
    tokio::time::timeout(
        Duration::from_secs(SONOBUOY_STATUS_TIMEOUT),
        wait_for_sonobuoy_status(kubeconfig_path, Some("testsys-workload")),
    )
    .await
    .context(error::SonobuoyTimeoutSnafu)??;
    info!("Workload status is available, waiting for test to complete");
    wait_for_sonobuoy_results(
        kubeconfig_path,
        Some("testsys-workload"),
        info_client,
        &workload_config.assume_role,
        aws_secret_name,
    )
    .await?;
    info!("Workload testing has completed, checking results");

    results_workload(kubeconfig_path, results_dir)
}

/// Reruns the the failed tests from workload conformance that has already run in this agent.
pub async fn rerun_failed_workload<I>(
    kubeconfig_path: &str,
    results_dir: &Path,
    info_client: &I,
    workload_config: &WorkloadConfig,
    aws_secret_name: &Option<&SecretName>,
) -> Result<TestResults, error::Error>
where
    I: InfoClient,
{
    let kubeconfig_arg = vec!["--kubeconfig", kubeconfig_path];
    let results_filepath = results_dir.join(SONOBUOY_RESULTS_FILENAME);

    let sonobuoy_image_arg = match &workload_config.sonobuoy_image {
        Some(sonobuoy_image_arg) => {
            vec!["--sonobuoy-image", sonobuoy_image_arg]
        }
        None => {
            vec![]
        }
    };

    info!("Rerunning workload");
    let output = Command::new(SONOBUOY_BIN_PATH)
        .args(kubeconfig_arg.to_owned())
        .arg("run")
        .arg("--namespace")
        .arg("testsys-workload")
        .args(sonobuoy_image_arg)
        .arg("--rerun-failed")
        .arg(results_filepath.as_os_str())
        .output()
        .context(error::WorkloadProcessSnafu)?;

    ensure!(
        output.status.success(),
        error::WorkloadRunSnafu {
            exit_code: output.status.code().unwrap_or_default(),
            stdout: &String::from_utf8(output.stdout).unwrap_or_else(|_| "".to_string()),
            stderr: &String::from_utf8(output.stderr).unwrap_or_else(|_| "".to_string()),
        }
    );

    info!("Workload testing has started, waiting for status to be available");
    tokio::time::timeout(
        Duration::from_secs(SONOBUOY_STATUS_TIMEOUT),
        wait_for_sonobuoy_status(kubeconfig_path, Some("testsys-workload")),
    )
    .await
    .context(error::SonobuoyTimeoutSnafu)??;
    info!("Workload status is available, waiting for test to complete");
    wait_for_sonobuoy_results(
        kubeconfig_path,
        Some("testsys-workload"),
        info_client,
        &workload_config.assume_role,
        aws_secret_name,
    )
    .await?;
    info!("Workload testing has completed, checking results");

    results_workload(kubeconfig_path, results_dir)
}

/// Retrieve the results from a workload test and convert them into `TestResults`.
pub fn results_workload(
    kubeconfig_path: &str,
    results_dir: &Path,
) -> Result<TestResults, error::Error> {
    let kubeconfig_arg = vec!["--kubeconfig", kubeconfig_path];

    info!("Running workload retrieve");
    let results_filepath = results_dir.join(SONOBUOY_RESULTS_FILENAME);
    let output = Command::new(SONOBUOY_BIN_PATH)
        .args(kubeconfig_arg.to_owned())
        .arg("retrieve")
        .arg("--namespace")
        .arg("testsys-workload")
        .arg("--filename")
        .arg(results_filepath.as_os_str())
        .output()
        .context(error::WorkloadProcessSnafu)?;

    ensure!(
        output.status.success(),
        error::WorkloadRunSnafu {
            exit_code: output.status.code().unwrap_or_default(),
            stdout: &String::from_utf8(output.stdout).unwrap_or_else(|_| "".to_string()),
            stderr: &String::from_utf8(output.stderr).unwrap_or_else(|_| "".to_string()),
        }
    );

    info!("Getting Workload status");
    let run_result = Command::new(SONOBUOY_BIN_PATH)
        .args(kubeconfig_arg)
        .arg("status")
        .arg("--json")
        .arg("--namespace")
        .arg("testsys-workload")
        .output()
        .context(error::WorkloadProcessSnafu)?;

    let stdout = String::from_utf8_lossy(&run_result.stdout);
    info!("Parsing the following workload results output:\n{}", stdout);

    trace!("Parsing workload results as json");
    let run_status: serde_json::Value =
        serde_json::from_str(&stdout).context(error::DeserializeJsonSnafu)?;
    trace!("The workload results are valid json");

    process_sonobuoy_test_results(&run_status)
}

/// Deletes all workload namespaces and associated resources in the target K8s cluster
pub async fn delete_workload(kubeconfig_path: &str) -> Result<(), error::Error> {
    let kubeconfig_arg = vec!["--kubeconfig", kubeconfig_path];
    info!("Deleting workload resources from cluster");
    let status = Command::new(SONOBUOY_BIN_PATH)
        .args(kubeconfig_arg)
        .arg("delete")
        .arg("--namespace")
        .arg("testsys-workload")
        .arg("--wait")
        .status()
        .context(error::WorkloadProcessSnafu)?;
    ensure!(status.success(), error::WorkloadDeleteSnafu);

    Ok(())
}
