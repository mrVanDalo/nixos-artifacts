//! Command-line argument definitions using clap.
//!
//! This module defines the CLI interface for the artifacts command.
//! All arguments are defined using clap derive macros for type safety
//! and automatic help generation.
//!
//! # Usage Examples
//!
//! ```bash
//! # Run TUI with default settings
//! artifacts
//!
//! # Filter by machine
//! artifacts --machine server-1
//!
//! # Filter by home-manager user
//! artifacts --home alice@host
//!
//! # Filter by artifact name
//! artifacts --artifact ssh-key
//!
//! # Combined filters
//! artifacts --machine server-1 --artifact ssh-key
//!
//! # Specify flake directory
//! artifacts /path/to/flake
//!
//! # Disable emoji output
//! artifacts --no-emoji
//! ```

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Logging level for debug output.
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum LogLevel {
    /// Error messages only
    Error,
    /// Warnings and errors
    Warn,
    /// Informational messages
    Info,
    /// Debug output (most verbose)
    Debug,
}

/// Command-line arguments for the artifacts CLI.
///
/// Parsed using clap derive macros. Use `--help` for full documentation.
#[derive(Debug, Parser)]
#[command(
    name = "artifacts",
    version,
    about = "TUI for managing NixOS Artifacts",
    long_about = None
)]
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
    #[arg(long = "log-file", value_name = "PATH")]
    pub log_file: Option<PathBuf>,

    /// Set the logging level (requires --log-file)
    #[arg(long = "log-level", value_enum, default_value_t = LogLevel::Debug)]
    pub log_level: LogLevel,
}

impl Cli {
    /// Returns true if logging is enabled (log_file is Some)
    pub fn is_logging_enabled(&self) -> bool {
        self.log_file.is_some()
    }
}
