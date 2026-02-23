//! Artifacts CLI - Main entry point
//!
//! This binary provides the `artifacts` command for managing NixOS artifacts
//! through interactive TUI, headless generation, and artifact listing.
//!
//! ## Commands
//!
//! - `artifacts` or `artifacts tui` - Launch interactive TUI for artifact management
//! - `artifacts list` - List all configured artifacts
//! - `artifacts generate` - Generate artifacts in headless mode
//!
//! ## Configuration
//!
//! The CLI requires:
//! - A `flake.nix` in the current directory (or specified via `--flake`)
//! - A `backend.toml` defining serialization backends (agenix, sops-nix, etc.)
//!
//! ## Exit Codes
//!
//! - **0** - Success
//! - **1** - General error
//!
//! ## Logging
//!
//! When the `logging` feature is enabled, errors are written to the log file
//! before being printed to stderr. See the `--log` argument for log file path.

#[cfg(feature = "logging")]
use log::error;

/// Main entry point for the artifacts CLI.
///
/// Initializes the CLI, parses arguments, and runs the appropriate command.
/// On error, prints the error to stderr and exits with code 1.
#[tokio::main]
async fn main() {
    if let Err(err) = artifacts::cli::run().await {
        #[cfg(feature = "logging")]
        error!("{:#}", err);
        #[cfg(not(feature = "logging"))]
        eprintln!("{:#}", err);
        std::process::exit(1);
    }
}
