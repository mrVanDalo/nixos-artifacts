//! Process execution with output capture and timeout support.
//!
//! This module provides functionality for executing child processes while capturing
//! their stdout and stderr output. It supports both standard execution and
//! timeout-protected execution, merging output streams in approximate arrival order.
//!
//! # Key Features
//!
//! - **Concurrent stream reading**: Uses separate threads to read stdout and stderr
//!   simultaneously, preserving output ordering via channels
//! - **Timeout protection**: Configurable timeouts prevent hanging on slow/misconfigured scripts
//! - **Error classification**: Different error types for timeout, I/O failures, and script failures
//! - **Output capture**: Complete capture of both stdout and stderr lines
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
//! ```
//!
//! For timeout-protected execution:
//! ```rust,ignore
//! let child = Command::new("slow_script.sh")
//!     .stdout(Stdio::piped())
//!     .stderr(Stdio::piped())
//!     .spawn()?;
//! let output = run_with_captured_output_and_timeout(
//!     child,
//!     "slow_script",
//!     Duration::from_secs(30)
//! )?;
//! ```

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
    },
    /// Script failed with non-zero exit code.
    ///
    /// The script executed but returned a non-zero exit status.
    Failed { exit_code: i32, stderr: String },
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
            } => {
                write!(
                    f,
                    "Script '{}' timed out after {} seconds",
                    script_name, timeout_secs
                )
            }
            ScriptError::Failed { exit_code, stderr } => {
                write!(f, "Script failed with exit code {}: {}", exit_code, stderr)
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
/// including all captured lines from stdout and stderr, and the
/// final exit status.
#[derive(Debug, Clone, Default)]
pub struct CapturedOutput {
    /// Collection of captured output lines with their source streams.
    ///
    /// Lines are stored in approximate order of arrival, with each
    /// line tagged by which stream (stdout/stderr) it came from.
    pub lines: Vec<OutputLine>,
    /// Whether the process exited successfully (exit code 0).
    pub exit_success: bool,
}

impl std::fmt::Display for CapturedOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = self
            .lines
            .iter()
            .map(|line| line.content.as_str())
            .collect::<Vec<&str>>()
            .join("\n");
        write!(f, "{}", content)
    }
}

/// A single line of output with its source stream.
///
/// Represents one line of output from either stdout or stderr,
/// preserving information about which stream it originated from.
#[derive(Debug, Clone)]
pub struct OutputLine {
    /// Which stream this line came from (stdout or stderr)
    pub stream: OutputStream,
    /// The content of the output line
    pub content: String,
}

/// Which stream the output came from.
///
/// Distinguishes between stdout and stderr output streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    /// Standard output stream
    Stdout,
    /// Standard error stream
    Stderr,
}

/// Run a child process and capture its stdout/stderr output.
///
/// Uses separate threads to read both streams concurrently, merging
/// them in approximate arrival order via a channel. This preserves
/// the relative ordering of lines between stdout and stderr.
///
/// # Arguments
///
/// * `child` - A spawned child process with piped stdout and stderr
///
/// # Returns
///
/// Returns a `CapturedOutput` containing all captured lines and the exit status.
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
/// assert_eq!(output.lines.len(), 1);
/// ```
pub fn run_with_captured_output(mut child: Child) -> std::io::Result<CapturedOutput> {
    let stdout = child.stdout.take().expect("stdout not piped");
    let stderr = child.stderr.take().expect("stderr not piped");

    let (tx, rx) = mpsc::channel();
    let tx_stdout = tx.clone();
    let tx_stderr = tx;

    // Spawn thread for stdout
    let stdout_thread = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            let _ = tx_stdout.send(OutputLine {
                stream: OutputStream::Stdout,
                content: line,
            });
        }
    });

    // Spawn thread for stderr
    let stderr_thread = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            let _ = tx_stderr.send(OutputLine {
                stream: OutputStream::Stderr,
                content: line,
            });
        }
    });

    // Collect lines in arrival order
    let mut lines = Vec::new();
    while let Ok(line) = rx.recv() {
        lines.push(line);
    }

    // Wait for threads to complete
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    let status = child.wait()?;

    Ok(CapturedOutput {
        lines,
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

    // Get child ID before spawning threads
    let child_id = child.id();

    let (tx, rx) = mpsc::channel::<OutputLine>();
    let tx_stdout = tx.clone();
    let tx_stderr = tx;

    // Spawn thread for stdout
    let stdout_thread = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if tx_stdout
                .send(OutputLine {
                    stream: OutputStream::Stdout,
                    content: line,
                })
                .is_err()
            {
                break;
            }
        }
    });

    // Spawn thread for stderr
    let stderr_thread = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            if tx_stderr
                .send(OutputLine {
                    stream: OutputStream::Stderr,
                    content: line,
                })
                .is_err()
            {
                break;
            }
        }
    });

    // Collect lines with timeout
    let mut lines = Vec::new();
    let start_time = std::time::Instant::now();
    let timeout_duration = timeout;

    loop {
        let elapsed = start_time.elapsed();
        if elapsed >= timeout_duration {
            // Timeout occurred - kill the process
            let _ = std::process::Command::new("kill")
                .arg("-9")
                .arg(child_id.to_string())
                .output();
            // Wait for threads to complete
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            // Wait for child to actually terminate
            let _ = child.wait();
            return Err(ScriptError::Timeout {
                script_name: script_name.to_string(),
                timeout_secs: timeout.as_secs(),
            });
        }

        let remaining = timeout_duration - elapsed;
        match rx.recv_timeout(remaining) {
            Ok(line) => lines.push(line),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Check if process has exited
                match child.try_wait() {
                    Ok(Some(_)) => break, // Process exited
                    Ok(None) => continue, // Still running, check again
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

    // Wait for threads to complete
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    // Get final status
    let status = match child.try_wait() {
        Ok(Some(status)) => status,
        Ok(None) => {
            // Still running, wait for it
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

    Ok(CapturedOutput {
        lines,
        exit_success: status.success(),
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
        assert_eq!(output.lines.len(), 1);
        assert_eq!(output.lines[0].stream, OutputStream::Stdout);
        assert_eq!(output.lines[0].content, "hello");
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
        assert_eq!(output.lines.len(), 1);
        assert_eq!(output.lines[0].stream, OutputStream::Stderr);
        assert_eq!(output.lines[0].content, "error");
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
        assert_eq!(output.lines.len(), 2);

        let stdout_lines: Vec<_> = output
            .lines
            .iter()
            .filter(|l| l.stream == OutputStream::Stdout)
            .collect();
        let stderr_lines: Vec<_> = output
            .lines
            .iter()
            .filter(|l| l.stream == OutputStream::Stderr)
            .collect();

        assert_eq!(stdout_lines.len(), 1);
        assert_eq!(stderr_lines.len(), 1);
        assert_eq!(stdout_lines[0].content, "out");
        assert_eq!(stderr_lines[0].content, "err");
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
}
