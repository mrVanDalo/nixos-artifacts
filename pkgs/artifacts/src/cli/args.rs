use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Parser)]
#[command(name = "artifacts", version, about = "TUI for managing NixOS Artifacts", long_about = None)]
pub struct Cli {
    /// Path to flake directory (default: current directory)
    pub flake: Option<PathBuf>,

    /// Filter by machine name (repeatable)
    #[arg(long = "machine")]
    pub machine: Vec<String>,

    /// Filter by home-manager user (repeatable)
    #[arg(long = "home")]
    pub home: Vec<String>,

    /// Filter by artifact name (repeatable)
    #[arg(long = "artifact")]
    pub artifact: Vec<String>,

    /// Disable emoji output
    #[arg(long = "no-emoji")]
    pub no_emoji: bool,

    /// Path to log file for debug output
    #[cfg(feature = "logging")]
    #[arg(long = "log-file", value_name = "PATH")]
    pub log_file: Option<PathBuf>,

    /// Set the logging level (requires --log-file)
    #[cfg(feature = "logging")]
    #[arg(long = "log-level", value_enum, default_value_t = LogLevel::Debug)]
    pub log_level: LogLevel,
}

impl Cli {
    /// Returns true if logging is enabled (log_file is Some)
    #[cfg(feature = "logging")]
    pub fn is_logging_enabled(&self) -> bool {
        self.log_file.is_some()
    }

    /// Returns false when logging feature is disabled
    #[cfg(not(feature = "logging"))]
    pub fn is_logging_enabled(&self) -> bool {
        false
    }
}
