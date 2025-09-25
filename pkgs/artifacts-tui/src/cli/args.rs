use clap::{Parser, Subcommand, ValueEnum};
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
#[command(name = "artifacts", version, about = "command line interafce to managing NixOS Artifacts", long_about = None)]
pub struct Cli {
    /// Set the logging level (error, warning, info, debug, trace)
    #[arg(long = "log-level", value_enum, default_value_t = LogLevel::Info)]
    pub log_level: LogLevel,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate artifacts
    ///
    /// Note: backend configuration is read from env var NIXOS_ARTIFACTS_BACKEND_CONFIG (path to backend.toml)
    Generate {
        /// Path to make configuration file (make.json)
        make: PathBuf,
        /// Regenerate all artifacts from all machines (conflicts with --machine/--artifact)
        #[arg(long = "all")]
        all: bool,
        /// Name of machine(s) to target (repeatable)
        #[arg(long = "machine")]
        machine: Vec<String>,
        /// Name of artifact(s) to target (repeatable)
        #[arg(long = "artifact")]
        artifact: Vec<String>,
    },
    /// Regenerate selected artifacts (or all)
    Regenerate {
        /// Path to backend configuration file (backend.toml)
        backend: PathBuf,
        /// Path to make configuration file (make.json)
        make: PathBuf,
        /// Regenerate all artifacts from all machines (conflicts with --machine/--artifact)
        #[arg(long = "all")]
        all: bool,
        /// Name of machine(s) to target (repeatable)
        #[arg(long = "machine")]
        machine: Vec<String>,
        /// Name of artifact(s) to target (repeatable)
        #[arg(long = "artifact")]
        artifact: Vec<String>,
    },
    /// List all machines and artifacts configured in make.json
    List {
        /// Path to backend configuration file (backend.toml)
        backend: PathBuf,
        /// Path to make configuration file (make.json)
        make: PathBuf,
    },
}
