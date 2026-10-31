//! Thin CLI helper for Minutes backends that shell out to local tools.
//!
//! [`CmdClient`] stores a program name, optional default arguments, a timeout,
//! and a [`CommandRunner`]. Call [`CmdClient::run`] with additional arguments;
//! the runner executes the program and returns stdout bytes. Failures map to
//! [`CmdClientError`].

use std::process::Output;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;
use tokio::process::Command;
use tokio::time::timeout;

use crate::http::DEFAULT_TIMEOUT;

/// Errors produced while building or executing CLI commands.
#[derive(Error, Debug)]
pub enum CmdClientError {
    /// Program name was empty.
    #[error("command program is required")]
    CommandRequired,
    /// The process could not be spawned.
    #[error("failed to spawn command: {0}")]
    Spawn(#[source] std::io::Error),
    /// Reading process output failed.
    #[error("failed to read command output: {0}")]
    Io(#[source] std::io::Error),
    /// The command did not finish within the configured timeout.
    #[error("command timed out after {0:?}")]
    Timeout(Duration),
    /// The command exited with a non-zero status.
    #[error("command exited with status {status}: {stderr}")]
    NonZeroExit {
        /// Process exit status code, if available.
        status: i32,
        /// Captured stderr text.
        stderr: String,
    },
}

/// Executes an external program and returns its stdout.
#[async_trait]
pub trait CommandRunner: Send + Sync {
    /// Runs `program` with `args` and returns stdout on success.
    ///
    /// # Errors
    ///
    /// Returns a [`CmdClientError`] when spawning fails, the process exits
    /// non-zero, or I/O fails.
    async fn output(&self, program: &str, args: &[String]) -> Result<Vec<u8>, CmdClientError>;
}

/// Default runner that spawns processes via [`tokio::process::Command`].
#[derive(Debug, Default, Clone, Copy)]
pub struct TokioCommandRunner;

#[async_trait]
impl CommandRunner for TokioCommandRunner {
    async fn output(&self, program: &str, args: &[String]) -> Result<Vec<u8>, CmdClientError> {
        let mut command = Command::new(program);
        command.args(args).kill_on_drop(true);

        let output = command.output().await.map_err(CmdClientError::Spawn)?;

        map_output(output)
    }
}

/// CLI client bound to a single program name.
///
/// Construct with [`CmdClient::new`] or [`CmdClient::with_default_args`], then
/// use [`CmdClient::run`] to execute. Tests can inject a custom runner with
/// [`CmdClient::with_runner`].
#[derive(Clone)]
pub struct CmdClient {
    program: String,
    default_args: Vec<String>,
    timeout: Duration,
    runner: Arc<dyn CommandRunner>,
}

impl std::fmt::Debug for CmdClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CmdClient")
            .field("program", &self.program)
            .field("default_args", &self.default_args)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

impl CmdClient {
    /// Creates a client for `program` with no default arguments.
    ///
    /// # Errors
    ///
    /// Returns [`CmdClientError::CommandRequired`] when `program` is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use minutes_client::cmd::CmdClient;
    ///
    /// let client = CmdClient::new("timew").unwrap();
    /// ```
    pub fn new(program: impl Into<String>) -> Result<Self, CmdClientError> {
        Self::with_default_args(program, Vec::<String>::new())
    }

    /// Creates a client for `program` that prepends `default_args` on every run.
    ///
    /// # Errors
    ///
    /// Returns [`CmdClientError::CommandRequired`] when `program` is empty.
    pub fn with_default_args<I, S>(
        program: impl Into<String>,
        default_args: I,
    ) -> Result<Self, CmdClientError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let program = program.into();
        if program.is_empty() {
            return Err(CmdClientError::CommandRequired);
        }

        Ok(Self {
            program,
            default_args: default_args.into_iter().map(Into::into).collect(),
            timeout: DEFAULT_TIMEOUT,
            runner: Arc::new(TokioCommandRunner),
        })
    }

    /// Returns the configured program name.
    #[must_use]
    pub fn program(&self) -> &str {
        &self.program
    }

    /// Replaces the command runner (useful for tests).
    #[must_use]
    pub fn with_runner(mut self, runner: Arc<dyn CommandRunner>) -> Self {
        self.runner = runner;
        self
    }

    /// Overrides the per-invocation timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Replaces the default arguments prepended on every [`CmdClient::run`].
    #[must_use]
    pub fn with_replaced_default_args<I, S>(mut self, default_args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.default_args = default_args.into_iter().map(Into::into).collect();
        self
    }

    /// Runs the configured program with `default_args` followed by `args`.
    ///
    /// # Errors
    ///
    /// Returns [`CmdClientError::Timeout`] when the runner does not finish in
    /// time, or any error from the underlying [`CommandRunner`].
    pub async fn run<I, S>(&self, args: I) -> Result<Vec<u8>, CmdClientError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut full_args = self.default_args.clone();
        full_args.extend(args.into_iter().map(|arg| arg.as_ref().to_owned()));

        match timeout(self.timeout, self.runner.output(&self.program, &full_args)).await {
            Ok(result) => result,
            Err(_) => Err(CmdClientError::Timeout(self.timeout)),
        }
    }
}

fn map_output(output: Output) -> Result<Vec<u8>, CmdClientError> {
    if output.status.success() {
        return Ok(output.stdout);
    }

    let status = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    Err(CmdClientError::NonZeroExit { status, stderr })
}

#[cfg(test)]
fn clone_error(error: &CmdClientError) -> CmdClientError {
    match error {
        CmdClientError::CommandRequired => CmdClientError::CommandRequired,
        CmdClientError::Timeout(duration) => CmdClientError::Timeout(*duration),
        CmdClientError::NonZeroExit { status, stderr } => CmdClientError::NonZeroExit {
            status: *status,
            stderr: stderr.clone(),
        },
        CmdClientError::Spawn(_) => CmdClientError::Spawn(std::io::Error::other("spawn failed")),
        CmdClientError::Io(_) => CmdClientError::Io(std::io::Error::other("io failed")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeRunner {
        calls: Mutex<Vec<(String, Vec<String>)>>,
        result: Mutex<Result<Vec<u8>, CmdClientError>>,
    }

    impl FakeRunner {
        fn ok(stdout: impl Into<Vec<u8>>) -> Arc<Self> {
            Arc::new(Self {
                calls: Mutex::new(Vec::new()),
                result: Mutex::new(Ok(stdout.into())),
            })
        }

        fn err(error: CmdClientError) -> Arc<Self> {
            Arc::new(Self {
                calls: Mutex::new(Vec::new()),
                result: Mutex::new(Err(error)),
            })
        }
    }

    #[async_trait]
    impl CommandRunner for FakeRunner {
        async fn output(&self, program: &str, args: &[String]) -> Result<Vec<u8>, CmdClientError> {
            self.calls
                .lock()
                .expect("lock")
                .push((program.to_owned(), args.to_vec()));
            self.result
                .lock()
                .expect("lock")
                .as_ref()
                .map_or_else(|err| Err(clone_error(err)), |bytes| Ok(bytes.clone()))
        }
    }

    struct HangingRunner;

    #[async_trait]
    impl CommandRunner for HangingRunner {
        async fn output(
            &self,
            _program: &str,
            _args: &[String],
        ) -> Result<Vec<u8>, CmdClientError> {
            std::future::pending().await
        }
    }

    #[test]
    fn new_rejects_empty_program() {
        let err = CmdClient::new("").expect_err("empty program");
        assert!(matches!(err, CmdClientError::CommandRequired));
    }

    #[tokio::test]
    async fn run_prepends_default_args() {
        let runner = FakeRunner::ok(b"ok");
        let client = CmdClient::with_default_args("timew", ["--verbose"])
            .expect("client")
            .with_runner(runner.clone());

        let stdout = client.run(["export", "from", "now"]).await.expect("run");
        assert_eq!(stdout, b"ok");

        let calls = runner.calls.lock().expect("lock");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "timew");
        assert_eq!(
            calls[0].1,
            vec![
                "--verbose".to_owned(),
                "export".to_owned(),
                "from".to_owned(),
                "now".to_owned()
            ]
        );
    }

    #[tokio::test]
    async fn run_maps_non_zero_exit() {
        let runner = FakeRunner::err(CmdClientError::NonZeroExit {
            status: 2,
            stderr: "boom".to_owned(),
        });
        let client = CmdClient::new("timew").expect("client").with_runner(runner);

        let err = client.run(["export"]).await.expect_err("non-zero");
        assert!(matches!(
            err,
            CmdClientError::NonZeroExit {
                status: 2,
                stderr
            } if stderr == "boom"
        ));
    }

    #[tokio::test]
    async fn run_times_out_when_runner_hangs() {
        let client = CmdClient::new("timew")
            .expect("client")
            .with_timeout(Duration::from_millis(20))
            .with_runner(Arc::new(HangingRunner));

        let err = client.run(["export"]).await.expect_err("timeout");
        assert!(matches!(err, CmdClientError::Timeout(_)));
    }
}
