use std::io::{BufRead, BufReader};
use std::process::Child;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Errors that can occur when running a script
#[derive(Debug, Clone)]
pub enum ScriptError {
    /// Script execution timed out
    Timeout {
        script_name: String,
        timeout_secs: u64,
    },
    /// Script failed with non-zero exit code
    Failed { exit_code: i32, stderr: String },
    /// I/O error occurred
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

/// Captured output from a child process
#[derive(Debug, Clone, Default)]
pub struct CapturedOutput {
    pub lines: Vec<OutputLine>,
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

/// A single line of output with its source stream
#[derive(Debug, Clone)]
pub struct OutputLine {
    pub stream: OutputStream,
    pub content: String,
}

/// Which stream the output came from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// Run a child process and capture its stdout/stderr output.
/// Uses separate threads to read both streams concurrently, merging
/// them in approximate arrival order via a channel.
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
/// If the timeout is reached, the child process is killed and a ScriptError::Timeout is returned.
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
