//! File-based debug logging for background task execution.
//!
//! This module provides file-based logging since stdout/stderr are lost when
//! the TUI starts. Logs are written to /tmp/artifacts_debug.log with timestamps.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

/// The log file path for debug output.
const LOG_FILE_PATH: &str = "/tmp/artifacts_debug.log";

/// Global log file handle protected by a mutex for thread safety.
/// Using OnceLock for lazy initialization without external dependencies.
use std::sync::OnceLock;

static LOG_FILE: OnceLock<Mutex<std::fs::File>> = OnceLock::new();

/// Initialize the log file handle.
/// Called automatically by log().
fn init_log_file() -> Mutex<std::fs::File> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .truncate(false)
        .open(LOG_FILE_PATH)
        .expect("Failed to open log file");
    Mutex::new(file)
}

/// Write a message to the debug log file with a timestamp.
///
/// # Arguments
///
/// * `msg` - The message to log
///
/// # Example
///
/// ```rust
/// logging::log("[RUNTIME] Spawning background task");
/// ```
pub fn log(msg: &str) {
    // Get or initialize the log file
    let file = LOG_FILE.get_or_init(init_log_file);

    // Get current time as HH:MM:SS.mmm
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let millis = now.as_millis();
    let seconds = (millis / 1000) % 86400; // Seconds since midnight
    let hours = (seconds / 3600) % 24;
    let minutes = (seconds / 60) % 60;
    let secs = seconds % 60;
    let ms = millis % 1000;

    let timestamp = format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, secs, ms);

    // Write to file with lock
    if let Ok(mut guard) = file.lock() {
        let _ = writeln!(guard, "[{}] {}", timestamp, msg);
        let _ = guard.flush();
    }
}

/// Log a formatted message with a prefix.
///
/// Convenience function for logging with a specific component prefix.
///
/// # Arguments
///
/// * `component` - The component name (e.g., "RUNTIME", "BACKGROUND")
/// * `msg` - The message to log
///
/// # Example
///
/// ```rust
/// logging::log_component("RUNTIME", "Spawning background task");
/// ```
pub fn log_component(component: &str, msg: &str) {
    log(&format!("[{}] {}", component, msg));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_writes_to_file() {
        // Clear log file first
        let _ = std::fs::remove_file(LOG_FILE_PATH);

        // Log a test message
        log("TEST: This is a test message");

        // Verify file exists and contains message
        let content = std::fs::read_to_string(LOG_FILE_PATH).expect("Log file should exist");
        assert!(content.contains("TEST: This is a test message"));
        assert!(content.contains("[")); // Should have timestamp
    }

    #[test]
    fn test_log_component() {
        // Clear log file first
        let _ = std::fs::remove_file(LOG_FILE_PATH);

        log_component("TEST", "Component message");

        let content = std::fs::read_to_string(LOG_FILE_PATH).expect("Log file should exist");
        assert!(content.contains("[TEST] Component message"));
    }
}
