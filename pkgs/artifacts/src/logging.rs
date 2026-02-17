//! File-based structured logging with feature-gated macro API.
//!
//! This module provides a complete logging infrastructure with:
//! - Four logging macros: error!, warn!, info!, debug!
//! - Logger struct for file-based output with real-time streaming
//! - Zero-cost when the "logging" feature is disabled (macros expand to nothing)
//! - Structured log format: [TIMESTAMP] [LEVEL] module: message
//!
//! # Usage
//!
//! ```rust
//! use artifacts::error;
//! use artifacts::warn;
//! use artifacts::info;
//! use artifacts::debug;
//!
//! error!("Failed to load configuration: {}", path);
//! warn!("Using default configuration");
//! info!("Started with {} artifacts", count);
//! debug!("Processing artifact: {:?}", artifact);
//! ```
//!
//! # Features
//!
//! When the `logging` feature is enabled:
//! - Macros call Logger::global().log() with structured output
//! - Logs include timestamps, module paths, and line numbers (DEBUG only)
//! - Logs are written to file specified by --log-file CLI argument
//!
//! When the `logging` feature is disabled:
//! - All macro calls are compiled away (zero runtime cost)
//! - No file I/O, no allocations, no overhead

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[cfg(feature = "logging")]
use std::fs::OpenOptions;

/// Log levels ordered by severity (lowest to highest for filtering)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Detailed debugging information
    Debug,
    /// General information
    Info,
    /// Warnings that don't prevent operation
    Warn,
    /// Errors that prevent operation
    Error,
}

impl LogLevel {
    /// Parse LogLevel from CLI LogLevel
    #[cfg(feature = "logging")]
    pub fn from_cli_level(level: &crate::cli::args::LogLevel) -> Self {
        use crate::cli::args::LogLevel as CliLevel;
        match level {
            CliLevel::Debug => LogLevel::Debug,
            CliLevel::Info => LogLevel::Info,
            CliLevel::Warn => LogLevel::Warn,
            CliLevel::Error => LogLevel::Error,
        }
    }
}

/// Logger that writes structured log entries to a file.
///
/// The Logger is designed for fail-fast operation - it validates writability
/// at creation time and streams logs in real-time (no buffering).
/// Logger that writes structured log entries to a file.
///
/// The Logger is designed for fail-fast operation - it validates writability
/// at creation time and streams logs in real-time (no buffering).
#[derive(Debug)]
pub struct Logger {
    /// The file handle, protected by a mutex for thread-safe writes
    file: Mutex<File>,
    /// Minimum level to log (filters out lower severity messages)
    min_level: LogLevel,
    /// Path to the log file
    path: PathBuf,
}

impl Logger {
    /// Create a new logger from CLI arguments.
    ///
    /// Returns `Ok(None)` if logging is not requested (--log-file not provided).
    /// Returns `Ok(Some(Logger))` when logging is enabled and file is writable.
    ///
    /// # Fail Fast Validation
    ///
    /// This function validates file writability at startup:
    /// - Checks if path is a directory (rejects)
    /// - Creates parent directories if needed
    /// - Verifies directory is writable with a test file
    /// - Opens the log file (overwrites existing)
    /// - Sets file permissions to 640 (owner read/write, group read)
    ///
    /// # Arguments
    ///
    /// * `args` - The parsed CLI arguments
    #[cfg(feature = "logging")]
    pub fn new_from_args(args: &crate::cli::args::Cli) -> anyhow::Result<Option<Self>> {
        let path = match &args.log_file {
            Some(p) => p,
            None => return Ok(None),
        };

        // Fail fast validation
        Self::validate_path(path)?;

        // Open file (overwrite mode, create dirs)
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .map_err(|e| anyhow::anyhow!("Failed to open log file: {}", e))?;

        // Set permissions to 640
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file.metadata()?.permissions();
            perms.set_mode(0o640);
            file.set_permissions(perms)?;
        }

        let min_level = LogLevel::from_cli_level(&args.log_level);

        Ok(Some(Self {
            file: Mutex::new(file),
            min_level,
            path: path.clone(),
        }))
    }

    /// Create a stub logger when logging feature is disabled.
    #[cfg(not(feature = "logging"))]
    pub fn new_from_args(_args: &crate::cli::args::Cli) -> anyhow::Result<Option<Self>> {
        Ok(None)
    }

    /// Validate that a path is writable (fail fast).
    ///
    /// Performs the following checks:
    /// 1. Path is not a directory
    /// 2. Parent directory exists or can be created
    /// 3. Parent directory is writable (tested with a temporary file)
    fn validate_path(path: &Path) -> anyhow::Result<()> {
        // Check if path is a directory
        if path.is_dir() {
            return Err(anyhow::anyhow!(
                "Log path cannot be a directory: {}",
                path.display()
            ));
        }

        // Check parent directory
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| anyhow::anyhow!("Failed to create log directory: {}", e))?;
            }

            if !parent.is_dir() {
                return Err(anyhow::anyhow!(
                    "Log path parent is not a directory: {}",
                    parent.display()
                ));
            }

            // Test writability by creating a temporary file
            let test_file = parent.join(".artifacts_log_test");
            match std::fs::File::create(&test_file) {
                Ok(_) => {
                    let _ = std::fs::remove_file(&test_file);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Cannot write to log directory '{}': {}",
                        parent.display(),
                        e
                    ));
                }
            }
        }

        Ok(())
    }

    /// Log a message at the specified level.
    ///
    /// The message is only written if the level is >= min_level.
    /// Each log entry is immediately flushed to disk (no buffering).
    ///
    /// Log format:
    /// - All levels: [TIMESTAMP] [LEVEL] module_path: message
    /// - Debug level: [TIMESTAMP] [LEVEL] module_path:line: message
    #[cfg(feature = "logging")]
    pub fn log(&self, level: LogLevel, module_path: &str, line: u32, message: String) {
        // Filter by level
        if level < self.min_level {
            return;
        }

        // Build log entry
        let timestamp = Self::format_timestamp();
        let level_str = format!("{:?}", level).to_uppercase();

        // Format: [TIMESTAMP] [LEVEL] module: message
        // At DEBUG level, include line number
        let entry = if level == LogLevel::Debug {
            format!(
                "[{}] [{}] {}:{}: {}",
                timestamp, level_str, module_path, line, message
            )
        } else {
            format!(
                "[{}] [{}] {}: {}",
                timestamp, level_str, module_path, message
            )
        };

        // Write with lock, flush immediately
        if let Ok(mut file) = self.file.lock() {
            let _ = writeln!(file, "{}", entry);
            let _ = file.flush();
        }
    }

    /// Format timestamp as HH:MM:SS.mmm
    fn format_timestamp() -> String {
        use std::time::SystemTime;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let millis = now.subsec_millis();

        let hours = (secs / 3600) % 24;
        let mins = (secs / 60) % 60;
        let secs = secs % 60;

        format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, millis)
    }

    /// Get the log file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the minimum log level.
    pub fn min_level(&self) -> LogLevel {
        self.min_level
    }
}

/// Global logger instance storage.
///
/// Uses OnceLock for thread-safe lazy initialization.
static GLOBAL_LOGGER: OnceLock<Option<Logger>> = OnceLock::new();

/// Initialize the global logger from CLI arguments.
///
/// This should be called once at application startup.
/// If a logger already exists, this returns an error.
#[cfg(feature = "logging")]
pub fn init_from_args(args: &crate::cli::args::Cli) -> anyhow::Result<()> {
    let logger = Logger::new_from_args(args)?;

    // Store in global - this will fail if already initialized
    GLOBAL_LOGGER
        .set(logger)
        .map_err(|_| anyhow::anyhow!("Logger already initialized"))?;

    Ok(())
}

/// No-op initialization when logging feature is disabled.
#[cfg(not(feature = "logging"))]
pub fn init_from_args(_args: &crate::cli::args::Cli) -> anyhow::Result<()> {
    Ok(())
}

/// Access the global logger instance.
///
/// Returns `Some(&Logger)` if logging is enabled and initialized.
/// Returns `None` if logging is disabled or not yet initialized.
pub fn global() -> Option<&'static Logger> {
    GLOBAL_LOGGER.get()?.as_ref()
}

// ============================================================================
// Legacy API (for backward compatibility)
// ============================================================================

/// Write a message to the log file (legacy API).
///
/// # Deprecated
/// This function is deprecated. Use the `info!` macro instead.
///
/// When the `logging` feature is enabled, writes the message to the log file
/// at INFO level with the "LEGACY" component prefix.
/// When disabled, does nothing.
pub fn log(msg: &str) {
    #[cfg(feature = "logging")]
    if let Some(logger) = global() {
        logger.log(LogLevel::Info, "legacy", 0, msg.to_string());
    }
}

/// Log a message with a component prefix (legacy API).
///
/// # Deprecated
/// This function is deprecated. Use the `info!` macro instead.
///
/// When the `logging` feature is enabled, writes the message to the log file
/// at INFO level with the specified component prefix.
/// When disabled, does nothing.
pub fn log_component(component: &str, msg: &str) {
    #[cfg(feature = "logging")]
    if let Some(logger) = global() {
        logger.log(LogLevel::Info, component, 0, msg.to_string());
    }
}

// ============================================================================
// Feature-gated logging macros
// ============================================================================

/// Log an error message.
///
/// When the `logging` feature is enabled, writes to the log file.
/// When disabled, compiles to nothing (zero cost).
///
/// # Examples
///
/// ```rust
/// error!("Failed to load file: {}", path);
/// error!("Configuration error: {:?}", err);
/// ```
#[cfg(feature = "logging")]
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        if let Some(logger) = $crate::logging::global() {
            logger.log(
                $crate::logging::LogLevel::Error,
                module_path!(),
                line!(),
                format!($($arg)*)
            )
        }
    }};
}

/// Log a warning message.
///
/// When the `logging` feature is enabled, writes to the log file.
/// When disabled, compiles to nothing (zero cost).
///
/// # Examples
///
/// ```rust
/// warn!("Using default configuration");
/// warn!("Deprecated feature: {}", feature_name);
/// ```
#[cfg(feature = "logging")]
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        if let Some(logger) = $crate::logging::global() {
            logger.log(
                $crate::logging::LogLevel::Warn,
                module_path!(),
                line!(),
                format!($($arg)*)
            )
        }
    }};
}

/// Log an informational message.
///
/// When the `logging` feature is enabled, writes to the log file.
/// When disabled, compiles to nothing (zero cost).
///
/// # Examples
///
/// ```rust
/// info!("Starting application");
/// info!("Loaded {} artifacts", count);
/// ```
#[cfg(feature = "logging")]
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        if let Some(logger) = $crate::logging::global() {
            logger.log(
                $crate::logging::LogLevel::Info,
                module_path!(),
                line!(),
                format!($($arg)*)
            )
        }
    }};
}

/// Log a debug message with module path and line number.
///
/// When the `logging` feature is enabled, writes to the log file.
/// When disabled, compiles to nothing (zero cost).
///
/// Debug messages include the line number for precise source location.
///
/// # Examples
///
/// ```rust
/// debug!("Processing artifact: {:?}", artifact);
/// debug!("State: {} files, {} prompts", file_count, prompt_count);
/// ```
#[cfg(feature = "logging")]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        if let Some(logger) = $crate::logging::global() {
            logger.log(
                $crate::logging::LogLevel::Debug,
                module_path!(),
                line!(),
                format!($($arg)*)
            )
        }
    }};
}

// Zero-cost macros when logging feature is disabled

/// No-op error macro when logging is disabled.
#[cfg(not(feature = "logging"))]
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {};
}

/// No-op warning macro when logging is disabled.
#[cfg(not(feature = "logging"))]
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {};
}

/// No-op info macro when logging is disabled.
#[cfg(not(feature = "logging"))]
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {};
}

/// No-op debug macro when logging is disabled.
#[cfg(not(feature = "logging"))]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {};
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::TempDir;

    // Import clap::Parser for Cli::parse_from
    use clap::Parser;

    #[test]
    fn test_log_level_ordering() {
        // Lower enum variants have lower values
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn test_log_level_from_cli() {
        use crate::cli::args::LogLevel as CliLevel;

        assert_eq!(LogLevel::from_cli_level(&CliLevel::Debug), LogLevel::Debug);
        assert_eq!(LogLevel::from_cli_level(&CliLevel::Info), LogLevel::Info);
        assert_eq!(LogLevel::from_cli_level(&CliLevel::Warn), LogLevel::Warn);
        assert_eq!(LogLevel::from_cli_level(&CliLevel::Error), LogLevel::Error);
    }

    #[test]
    fn test_logger_creation_without_log_file() {
        use crate::cli::args::Cli;

        // Create minimal CLI args without --log-file
        let args = Cli::parse_from(["artifacts"]);

        // Should return None
        #[cfg(feature = "logging")]
        {
            let result = Logger::new_from_args(&args).unwrap();
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_logger_validates_writability() {
        use crate::cli::args::Cli;

        // Create temp directory for log file
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        // Create CLI args with log file
        let args = Cli::parse_from([
            "artifacts",
            "--log-file",
            log_path.to_str().unwrap(),
            "--log-level",
            "debug",
        ]);

        #[cfg(feature = "logging")]
        {
            // Create logger
            let logger = Logger::new_from_args(&args)
                .unwrap()
                .expect("Logger should be created");
            assert_eq!(logger.path(), log_path);
            assert_eq!(logger.min_level(), LogLevel::Debug);

            // Check file was created with correct permissions
            let metadata = std::fs::metadata(&log_path).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = metadata.permissions().mode();
                // Should be 0o640 (owner rw, group r)
                assert_eq!(perms & 0o777, 0o640);
            }
        }
    }

    #[test]
    fn test_logger_rejects_directory() {
        use crate::cli::args::Cli;

        // Create temp directory
        let temp_dir = TempDir::new().unwrap();

        // Try to use directory as log file
        let args = Cli::parse_from(["artifacts", "--log-file", temp_dir.path().to_str().unwrap()]);

        #[cfg(feature = "logging")]
        {
            let result = Logger::new_from_args(&args);
            assert!(result.is_err());
            match result {
                Err(e) => {
                    let err = e.to_string();
                    assert!(err.contains("cannot be a directory"));
                }
                Ok(_) => panic!("Expected error"),
            }
        }
    }

    #[test]
    fn test_logger_writes_to_file() {
        use crate::cli::args::Cli;

        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let args = Cli::parse_from([
            "artifacts",
            "--log-file",
            log_path.to_str().unwrap(),
            "--log-level",
            "debug",
        ]);

        #[cfg(feature = "logging")]
        {
            let logger = Logger::new_from_args(&args).unwrap().unwrap();

            // Write a log entry
            logger.log(
                super::LogLevel::Info,
                "test_module",
                42,
                "Test message".to_string(),
            );

            // Read file and verify content
            let mut content = String::new();
            let mut file = File::open(&log_path).unwrap();
            file.read_to_string(&mut content).unwrap();

            assert!(content.contains("[INFO]"));
            assert!(content.contains("test_module"));
            assert!(content.contains("Test message"));
            assert!(!content.contains(":42:")); // INFO doesn't include line number
        }
    }

    #[test]
    fn test_logger_debug_includes_line_number() {
        use crate::cli::args::Cli;

        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let args = Cli::parse_from([
            "artifacts",
            "--log-file",
            log_path.to_str().unwrap(),
            "--log-level",
            "debug",
        ]);

        #[cfg(feature = "logging")]
        {
            let logger = Logger::new_from_args(&args).unwrap().unwrap();

            // Write debug log at specific line
            logger.log(
                super::LogLevel::Debug,
                "test_module",
                999,
                "Debug message".to_string(),
            );

            // Read file
            let mut content = String::new();
            let mut file = File::open(&log_path).unwrap();
            file.read_to_string(&mut content).unwrap();

            // DEBUG should include line number
            assert!(content.contains("[DEBUG]"));
            assert!(content.contains("test_module:999:"));
        }
    }

    #[test]
    fn test_level_filtering() {
        use crate::cli::args::Cli;

        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        // Set minimum level to WARN
        let args = Cli::parse_from([
            "artifacts",
            "--log-file",
            log_path.to_str().unwrap(),
            "--log-level",
            "warn",
        ]);

        #[cfg(feature = "logging")]
        {
            let logger = Logger::new_from_args(&args).unwrap().unwrap();

            // Write at different levels
            logger.log(super::LogLevel::Debug, "test", 1, "debug".to_string());
            logger.log(super::LogLevel::Info, "test", 2, "info".to_string());
            logger.log(super::LogLevel::Warn, "test", 3, "warn".to_string());
            logger.log(super::LogLevel::Error, "test", 4, "error".to_string());

            // Read file
            let mut content = String::new();
            let mut file = File::open(&log_path).unwrap();
            file.read_to_string(&mut content).unwrap();

            // Should only see WARN and ERROR
            assert!(!content.contains("debug"));
            assert!(!content.contains("info"));
            assert!(content.contains("warn"));
            assert!(content.contains("error"));
        }
    }

    #[test]
    fn test_format_timestamp() {
        let timestamp = Logger::format_timestamp();
        // Format should be HH:MM:SS.mmm
        assert!(timestamp.contains(':'));
        assert!(timestamp.contains('.'));
        assert_eq!(timestamp.len(), 12); // "HH:MM:SS.mmm"
    }

    #[test]
    fn test_macros_exist_with_feature() {
        // This test verifies the macros compile and are accessible
        // With feature enabled, they should work
        #[cfg(feature = "logging")]
        {
            // These should compile (don't execute since no logger initialized)
            // Just verifying syntax is valid
            let _ = || {
                crate::error!("test {}", 1);
                crate::warn!("test {}", 2);
                crate::info!("test {}", 3);
                crate::debug!("test {}", 4);
            };
        }

        // With feature disabled, macros should expand to nothing
        #[cfg(not(feature = "logging"))]
        {
            // These should compile and have no runtime cost
            crate::error!("test {}", 1);
            crate::warn!("test {}", 2);
            crate::info!("test {}", 3);
            crate::debug!("test {}", 4);
        }
    }

    #[test]
    fn test_global_logger_uninitialized() {
        // Before initialization, global() should return None
        assert!(global().is_none());
    }
}
