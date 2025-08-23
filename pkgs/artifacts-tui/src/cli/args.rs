use clap::{CommandFactory, Parser, Subcommand};
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_contains_generate() {
        let mut help = Vec::new();
        let _ = Cli::command().write_long_help(&mut help);
        let text = String::from_utf8(help).unwrap();
        assert!(text.contains("generate"));
    }
}
