use crate::error::{self, Error, Result};
use crate::{BootstrapData, Client, Runner};
use log::{debug, error};

/// The `TestAgent` is the main entrypoint for the program running in a TestPod. It starts a test
/// run, regularly checks the health of the test run, observes cancellation of a test run, and sends
/// the results of a test run.
///
/// To create a test, implement the [`Runner`] trait on an object and inject it into the
/// `TestAgent`.
///
/// Two additional dependencies are injected for the sake of testability. You can mock these traits
/// in order to test your [`Runner`] in the absence of k8s.
/// - [`Bootstrap`] collects information from the container environment.
/// - [`Client`] communicates with the k8s server.
///
/// See the `../examples/example_test_agent/main.rs` for an example of how to create a [`Runner`].
/// Also see `../tests/mock.rs` for an example of how you can mock the Kubernetes clients.
///
pub struct TestAgent<C, R>
where
    C: Client + 'static,
    R: Runner + 'static,
{
    client: C,
    runner: R,
}

impl<C, R> TestAgent<C, R>
where
    C: Client + 'static,
    R: Runner + 'static,
{
    /// Create a new `TestAgent`. Since the [`Client`] and [`Runner`] are constructed internally
    /// based on information from the [`BootstrapData`], you will need to specify the types using
    /// the type parameters. `TestAgent::<DefaultClient, MyRunner>::new(BootstrapData::from_env())`.
    /// Any errors that occur during this function are fatal since we are not able to fully
    /// construct the `Runner`.
    pub async fn new(b: BootstrapData) -> Result<Self, C::E, R::E> {
        let client = C::new(b).await.map_err(|e| Error::Client(e))?;
        let test_info = client.get_test_info().await.map_err(|e| Error::Client(e))?;
        let runner = R::new(test_info).await.map_err(|e| Error::Runner(e))?;
        Ok(Self { runner, client })
    }

    /// Run the `TestAgent`. This function returns once the test has completed.
    pub async fn run(&mut self) -> Result<(), C::E, R::E> {
        debug!("running test");
        self.client
            .send_test_starting()
            .await
            .map_err(|e| error::Error::Client(e))?;

        let test_results = match self.runner.run().await.map_err(|e| error::Error::Runner(e)) {
            Ok(ok) => ok,
            Err(e) => {
                self.send_error_best_effort(&e).await;
                self.terminate_best_effort().await;
                return Err(e);
            }
        };

        if let Err(e) = self
            .client
            .send_test_done(test_results)
            .await
            .map_err(|e| error::Error::Client(e))
        {
            self.send_error_best_effort(&e).await;
            self.terminate_best_effort().await;
            return Err(e);
        }

        // Test finished successfully. Try to terminate. If termination fails, we try to send the
        // error to k8s, and return the error so that the process will exit with error.
        if let Err(e) = self
            .runner
            .terminate()
            .await
            .map_err(|e| error::Error::Runner(e))
        {
            error!("unable to terminate test runner: {}", e);
            self.send_error_best_effort(&e).await;
            return Err(e);
        }

        Ok(())
    }

    /// Returns `true` if the error was successfully sent, `false` if the error could not be sent.
    async fn send_error_best_effort(&mut self, e: &Error<C::E, R::E>) {
        if let Err(send_error) = self.client.send_error(e).await {
            error!(
                "unable to send error message '{}' to k8s: {}",
                e, send_error
            );
        }
    }

    /// Tells the `Runner` to terminate. If an error occurs, tries to send it to k8s, but logs it
    /// if it cannot be sent to k8s.
    async fn terminate_best_effort(&mut self) {
        // TODO - stay running https://github.com/bottlerocket-os/bottlerocket-test-system/issues/79
        if let Err(e) = self
            .runner
            .terminate()
            .await
            .map_err(|e| error::Error::Runner(e))
        {
            self.send_error_best_effort(&e).await;
        }
    }
}
