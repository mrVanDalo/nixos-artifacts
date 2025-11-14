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

    /// don't use emojis
    #[arg(long = "no-emoji")]
    pub no_emoji: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate artifacts
    ///
    /// Backend configuration is read from the environment variable NIXOS_ARTIFACTS_BACKEND_CONFIG
    /// if set; otherwise, it falls back to <flake-dir>/backend.toml.
    Generate {
        /// Path to flake to read machines/artifacts from (passed as -I flake=<path> to nix). If omitted, uses the current directory.
        make: Option<PathBuf>,
        /// Regenerate all artifacts from all machines/users (conflicts with --machine/--home/--artifact)
        #[arg(long = "all")]
        all: bool,
        /// Name of machine(s) to target (repeatable)
        #[arg(long = "machine")]
        machine: Vec<String>,
        /// Name of home user(s) to target (repeatable)
        #[arg(long = "home")]
        home: Vec<String>,
        /// Name of artifact(s) to target (repeatable)
        #[arg(long = "artifact")]
        artifact: Vec<String>,
    },
    /// Regenerate selected artifacts (or all)
    ///
    /// Backend configuration is read from the environment variable NIXOS_ARTIFACTS_BACKEND_CONFIG
    /// if set; otherwise, it falls back to <flake-dir>/backend.toml.
    Regenerate {
        /// Path to flake to read machines/artifacts from (passed as -I flake=<path> to nix). If omitted, uses the current directory.
        make: Option<PathBuf>,
        /// Regenerate all artifacts from all machines/users (conflicts with --machine/--home/--artifact)
        #[arg(long = "all")]
        all: bool,
        /// Name of machine(s) to target (repeatable)
        #[arg(long = "machine")]
        machine: Vec<String>,
        /// Name of home user(s) to target (repeatable)
        #[arg(long = "home")]
        home: Vec<String>,
        /// Name of artifact(s) to target (repeatable)
        #[arg(long = "artifact")]
        artifact: Vec<String>,
    },
    /// List all machines and artifacts defined by the flake
    ///
    /// Backend configuration is read from the environment variable NIXOS_ARTIFACTS_BACKEND_CONFIG
    /// if set; otherwise, it falls back to <flake-dir>/backend.toml.
    List {
        /// Path to flake to read machines/artifacts from (passed as -I flake=<path> to nix). If omitted, uses the current directory.
        make: Option<PathBuf>,
    },
}
