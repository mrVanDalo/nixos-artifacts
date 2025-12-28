use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
    Trace,
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

    /// Set the logging level
    #[arg(long = "log-level", value_enum, default_value_t = LogLevel::Info)]
    pub log_level: LogLevel,

    /// Disable emoji output
    #[arg(long = "no-emoji")]
    pub no_emoji: bool,
}
