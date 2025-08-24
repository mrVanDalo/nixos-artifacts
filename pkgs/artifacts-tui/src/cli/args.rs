use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "artifacts-tui", version, about = "TUI for managing NixOS artifacts", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate artifacts
    Generate {
        /// Path to backend configuration file (backend.toml)
        backend: PathBuf,
        /// Path to make configuration file (make.json)
        make: PathBuf,
    },
}
