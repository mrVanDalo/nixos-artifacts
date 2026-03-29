//! Process execution with output capture and timeout support.
//!
//! This module provides functionality for executing child processes while capturing
//! their stdout and stderr output separately. It supports both standard execution and
//! timeout-protected execution.
//!
//! # Key Features
//!
//! - **Concurrent stream reading**: Uses separate threads to read stdout and stderr
//!   simultaneously
//! - **Timeout protection**: Configurable timeouts prevent hanging on slow/misconfigured scripts
//! - **Error classification**: Different error types for timeout, I/O failures, and script failures
//! - **Separate output capture**: stdout and stderr are captured in separate vectors
//!
//! # Usage
//!
//! For standard execution without timeout:
//! ```rust,ignore
//! let child = Command::new("script.sh")
//!     .stdout(Stdio::piped())
//!     .stderr(Stdio::piped())
//!     .spawn()?;
//! let output = run_with_captured_output(child)?;
//! // output.stdout contains stdout lines
//! // output.stderr contains stderr lines
//! ```
//!
//! For timeout-protected execution:
//! ```rust,ignore
//! let child = Command::new("slow_script.sh")
//!     .stdout(Stdio::piped())
//!     .stderr(Stdio::piped())
//!     .spawn()?;
//! match run_with_captured_output_and_timeout(
//!     child,
//!     "slow_script",
//!     Duration::from_secs(30)
//! ) {
//!     Ok(output) => {
//!         println!("stdout: {:?}", output.stdout);
//!         println!("stderr: {:?}", output.stderr);
//!     }
//!     Err(ScriptError::Timeout { stdout, stderr, .. }) => {
//!         // Output captured before timeout is still available
//!     }
//!     Err(ScriptError::Failed { stdout, stderr, .. }) => {
//!         // Both stdout and stderr available on failure
//!     }
//!     _ => {}
//! }
//! ```

pub use crate::app::model::OutputStream;
use std::io::{BufRead, BufReader};
use std::process::Child;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Errors that can occur when running a script.
///
/// This enum represents the various failure modes that can occur
/// during script execution, providing detailed context for each.
#[derive(Debug, Clone)]
pub enum ScriptError {
    /// Script execution timed out.
    ///
    /// The script exceeded the configured timeout duration and was terminated.
    Timeout {
        /// Name of the script that timed out
        script_name: String,
        /// Timeout duration in seconds
        timeout_secs: u64,
        /// Captured stdout before timeout
        stdout: String,
        /// Captured stderr before timeout
        stderr: String,
    },
    /// Script failed with non-zero exit code.
    ///
    /// The script executed but returned a non-zero exit status.
    Failed {
        exit_code: i32,
        stdout: String,
        stderr: String,
    },
    /// I/O error occurred during script execution.
    ///
    /// An I/O error occurred while trying to execute the script or
    /// capture its output.
    Io { message: String },
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptError::Timeout {
                script_name,
                timeout_secs,
                stdout,
                stderr,
            } => {
                write!(
                    f,
                    "Script '{}' timed out after {} seconds\nstdout: {}\nstderr: {}",
                    script_name,
                    timeout_secs,
                    if stdout.is_empty() { "(no output)" } else { stdout },
                    if stderr.is_empty() { "(no output)" } else { stderr }
                )
            }
            ScriptError::Failed {
                exit_code,
                stdout,
                stderr,
            } => {
                write!(
                    f,
                    "Script failed with exit code {}\nstdout: {}\nstderr: {}",
                    exit_code,
                    if stdout.is_empty() { "(no output)" } else { stdout },
                    if stderr.is_empty() { "(no output)" } else { stderr }
                )
            }
            ScriptError::Io { message } => {
                write!(f, "I/O error: {}", message)
            }
        }
    }
}

impl std::error::Error for ScriptError {}

/// Captured output from a child process.
///
/// Contains the complete output from a child process execution,
/// with stdout and stderr captured separately.
#[derive(Debug, Clone, Default)]
pub struct CapturedOutput {
    /// All captured stdout lines in order of arrival.
    pub stdout: Vec<String>,
    /// All captured stderr lines in order of arrival.
    pub stderr: Vec<String>,
    /// Whether the process exited successfully (exit code 0).
    pub exit_success: bool,
}

impl std::fmt::Display for CapturedOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stdout: {}\nstderr: {}",
            if self.stdout.is_empty() {
                "(no output)".to_string()
            } else {
                self.stdout.join("\n")
            },
            if self.stderr.is_empty() {
                "(no output)".to_string()
            } else {
                self.stderr.join("\n")
            }
        )
    }
}

/// Run a child process and capture its stdout/stderr output.
///
/// Uses separate threads to read both streams concurrently,
/// collecting stdout and stderr lines separately.
///
/// # Arguments
///
/// * `child` - A spawned child process with piped stdout and stderr
///
/// # Returns
///
/// Returns a `CapturedOutput` containing stdout lines, stderr lines, and the exit status.
///
/// # Panics
///
/// Panics if stdout or stderr are not piped. Ensure the child process
/// was spawned with `.stdout(Stdio::piped())` and `.stderr(Stdio::piped())`.
///
/// # Example
///
/// ```rust,ignore
/// let child = Command::new("echo")
///     .arg("hello")
///     .stdout(Stdio::piped())
///     .stderr(Stdio::piped())
///     .spawn()?;
///
/// let output = run_with_captured_output(child)?;
/// assert!(output.exit_success);
/// assert_eq!(output.stdout.len(), 1);
/// ```
pub fn run_with_captured_output(mut child: Child) -> std::io::Result<CapturedOutput> {
    let stdout = child.stdout.take().expect("stdout not piped");
    let stderr = child.stderr.take().expect("stderr not piped");

    let (tx, rx) = mpsc::channel();
    let tx_stdout = tx.clone();
    let tx_stderr = tx;

    let stdout_thread = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            let _ = tx_stdout.send((OutputStream::Stdout, line));
        }
    });

    let stderr_thread = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            let _ = tx_stderr.send((OutputStream::Stderr, line));
        }
    });

    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    while let Ok((stream, line)) = rx.recv() {
        match stream {
            OutputStream::Stdout => stdout_lines.push(line),
            OutputStream::Stderr => stderr_lines.push(line),
        }
    }

    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    let status = child.wait()?;

    Ok(CapturedOutput {
        stdout: stdout_lines,
        stderr: stderr_lines,
        exit_success: status.success(),
    })
}

/// Run a child process with a timeout and capture its stdout/stderr output.
///
/// Similar to `run_with_captured_output`, but enforces a timeout on the execution.
/// If the timeout is reached, the child process is killed and a `ScriptError::Timeout`
/// is returned. This prevents hanging on slow or misconfigured scripts.
///
/// # Arguments
///
/// * `child` - A spawned child process with piped stdout and stderr
/// * `script_name` - Name of the script (for error reporting)
/// * `timeout` - Maximum duration to wait for script completion
///
/// # Returns
///
/// Returns `Ok(CapturedOutput)` on success, or `Err(ScriptError)` if:
/// - The script times out
/// - An I/O error occurs
/// - The script fails with non-zero exit code
///
/// # Example
///
/// ```rust,ignore
/// let child = Command::new("slow_script.sh")
///     .stdout(Stdio::piped())
///     .stderr(Stdio::piped())
///     .spawn()?;
///
/// match run_with_captured_output_and_timeout(
///     child,
///     "slow_script",
///     Duration::from_secs(30)
/// ) {
///     Ok(output) if output.exit_success => println!("Success!"),
///     Ok(output) => println!("Script failed: {}", output),
///     Err(ScriptError::Timeout { .. }) => println!("Timed out!"),
///     Err(e) => println!("Error: {}", e),
/// }
/// ```
pub fn run_with_captured_output_and_timeout(
    mut child: Child,
    script_name: &str,
    timeout: Duration,
) -> Result<CapturedOutput, ScriptError> {
    let stdout = child.stdout.take().ok_or_else(|| ScriptError::Io {
        message: "stdout not piped".to_string(),
    })?;
    let stderr = child.stderr.take().ok_or_else(|| ScriptError::Io {
        message: "stderr not piped".to_string(),
    })?;

    let child_id = child.id();

    let (tx, rx) = mpsc::channel::<(OutputStream, String)>();
    let tx_stdout = tx.clone();
    let tx_stderr = tx;

    let stdout_thread = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if tx_stdout.send((OutputStream::Stdout, line)).is_err() {
                break;
            }
        }
    });

    let stderr_thread = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            if tx_stderr.send((OutputStream::Stderr, line)).is_err() {
                break;
            }
        }
    });

    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    let start_time = std::time::Instant::now();
    let timeout_duration = timeout;

    loop {
        let elapsed = start_time.elapsed();
        if elapsed >= timeout_duration {
            let _ = std::process::Command::new("kill")
                .arg("-9")
                .arg(child_id.to_string())
                .output();
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            let _ = child.wait();
            return Err(ScriptError::Timeout {
                script_name: script_name.to_string(),
                timeout_secs: timeout.as_secs(),
                stdout: stdout_lines.join("\n"),
                stderr: stderr_lines.join("\n"),
            });
        }

        let remaining = timeout_duration - elapsed;
        match rx.recv_timeout(remaining) {
            Ok((OutputStream::Stdout, line)) => stdout_lines.push(line),
            Ok((OutputStream::Stderr, line)) => stderr_lines.push(line),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => continue,
                    Err(e) => {
                        let _ = stdout_thread.join();
                        let _ = stderr_thread.join();
                        return Err(ScriptError::Io {
                            message: e.to_string(),
                        });
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    let status = match child.try_wait() {
        Ok(Some(status)) => status,
        Ok(None) => {
            child.wait().map_err(|e| ScriptError::Io {
                message: e.to_string(),
            })?
        }
        Err(e) => {
            return Err(ScriptError::Io {
                message: e.to_string(),
            });
        }
    };

    if !status.success() {
        return Err(ScriptError::Failed {
            exit_code: status.code().unwrap_or(-1),
            stdout: stdout_lines.join("\n"),
            stderr: stderr_lines.join("\n"),
        });
    }

    Ok(CapturedOutput {
        stdout: stdout_lines,
        stderr: stderr_lines,
        exit_success: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn test_capture_stdout_only() {
        let child = Command::new("echo")
            .arg("hello")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let output = run_with_captured_output(child).unwrap();
        assert!(output.exit_success);
        assert_eq!(output.stdout.len(), 1);
        assert_eq!(output.stdout[0], "hello");
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn test_capture_stderr_only() {
        let child = Command::new("sh")
            .arg("-c")
            .arg("echo error >&2")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let output = run_with_captured_output(child).unwrap();
        assert!(output.exit_success);
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr.len(), 1);
        assert_eq!(output.stderr[0], "error");
    }

    #[test]
    fn test_capture_both_streams() {
        let child = Command::new("sh")
            .arg("-c")
            .arg("echo out; echo err >&2")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let output = run_with_captured_output(child).unwrap();
        assert!(output.exit_success);
        assert_eq!(output.stdout.len(), 1);
        assert_eq!(output.stderr.len(), 1);
        assert_eq!(output.stdout[0], "out");
        assert_eq!(output.stderr[0], "err");
    }

    #[test]
    fn test_capture_exit_failure() {
        let child = Command::new("sh")
            .arg("-c")
            .arg("exit 1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let output = run_with_captured_output(child).unwrap();
        assert!(!output.exit_success);
    }

    #[test]
    fn test_timeout_captures_output() {
        let child = Command::new("sh")
            .arg("-c")
            .arg("echo stdout_msg; echo stderr_msg >&2; sleep 10")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let result = run_with_captured_output_and_timeout(
            child,
            "test_script",
            Duration::from_millis(100),
        );

        match result {
            Err(ScriptError::Timeout { stdout, stderr, .. }) => {
                assert_eq!(stdout, "stdout_msg");
                assert_eq!(stderr, "stderr_msg");
            }
            _ => panic!("Expected timeout error with captured output"),
        }
    }

    #[test]
    fn test_failed_script_captures_output() {
        let child = Command::new("sh")
            .arg("-c")
            .arg("echo stdout_content; echo stderr_content >&2; exit 42")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let result = run_with_captured_output_and_timeout(
            child,
            "test_script",
            Duration::from_secs(5),
        );

        match result {
            Err(ScriptError::Failed { exit_code, stdout, stderr }) => {
                assert_eq!(exit_code, 42);
                assert_eq!(stdout, "stdout_content");
                assert_eq!(stderr, "stderr_content");
            }
            _ => panic!("Expected Failed error with captured output"),
        }
    }

    #[test]
    fn test_failed_script_empty_output() {
        let child = Command::new("sh")
            .arg("-c")
            .arg("exit 1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let result = run_with_captured_output_and_timeout(
            child,
            "test_script",
            Duration::from_secs(5),
        );

        match result {
            Err(ScriptError::Failed { exit_code, stdout, stderr }) => {
                assert_eq!(exit_code, 1);
                assert!(stdout.is_empty());
                assert!(stderr.is_empty());
            }
            _ => panic!("Expected Failed error with empty output"),
        }
    }

    #[test]
    fn test_script_error_display_with_output() {
        let err = ScriptError::Failed {
            exit_code: 1,
            stdout: "stdout content".to_string(),
            stderr: "stderr content".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("exit code 1"));
        assert!(display.contains("stdout: stdout content"));
        assert!(display.contains("stderr: stderr content"));
    }

    #[test]
    fn test_script_error_display_empty_output() {
        let err = ScriptError::Failed {
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
        };
        let display = format!("{}", err);
        assert!(display.contains("exit code 1"));
        assert!(display.contains("stdout: (no output)"));
        assert!(display.contains("stderr: (no output)"));
    }

    #[test]
    fn test_timeout_error_display_with_output() {
        let err = ScriptError::Timeout {
            script_name: "test".to_string(),
            timeout_secs: 30,
            stdout: "partial stdout".to_string(),
            stderr: "partial stderr".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("timed out after 30 seconds"));
        assert!(display.contains("stdout: partial stdout"));
        assert!(display.contains("stderr: partial stderr"));
    }

    #[test]
    fn test_timeout_error_display_empty_output() {
        let err = ScriptError::Timeout {
            script_name: "test".to_string(),
            timeout_secs: 30,
            stdout: String::new(),
            stderr: String::new(),
        };
        let display = format!("{}", err);
        assert!(display.contains("timed out after 30 seconds"));
        assert!(display.contains("stdout: (no output)"));
        assert!(display.contains("stderr: (no output)"));
    }

    #[test]
    fn test_captured_output_display_with_content() {
        let output = CapturedOutput {
            stdout: vec!["line1".to_string(), "line2".to_string()],
            stderr: vec!["error".to_string()],
            exit_success: true,
        };
        let display = format!("{}", output);
        assert!(display.contains("stdout: line1\nline2"));
        assert!(display.contains("stderr: error"));
    }

    #[test]
    fn test_captured_output_display_empty() {
        let output = CapturedOutput {
            stdout: vec![],
            stderr: vec![],
            exit_success: true,
        };
        let display = format!("{}", output);
        assert!(display.contains("stdout: (no output)"));
        assert!(display.contains("stderr: (no output)"));
    }
}
