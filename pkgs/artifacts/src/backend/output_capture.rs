use std::io::{BufRead, BufReader};
use std::process::Child;
use std::sync::mpsc;
use std::thread;

/// Captured output from a child process
#[derive(Debug, Clone, Default)]
pub struct CapturedOutput {
    pub lines: Vec<OutputLine>,
    pub exit_success: bool,
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
