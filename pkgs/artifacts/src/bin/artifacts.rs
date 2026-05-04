//! Artifacts CLI - Main entry point
//!
//! This binary provides the `artifacts` command, an interactive TUI for
//! managing NixOS artifacts. The TUI is the only mode; there are no
//! subcommands.
//!
//! ## Usage
//!
//! - `artifacts` — Launch the TUI against the flake in the current directory
//! - `artifacts /path/to/flake` — Launch the TUI against a different flake
//!
//! ## Configuration
//!
//! The CLI requires:
//! - A `flake.nix` in the current directory (or in the directory passed as the
//!   positional argument)
//! - A `backend.toml` defining serialization backends (agenix, sops-nix, etc.)
//!
//! ## Exit Codes
//!
//! - **0** - The TUI ran to completion. Per-artifact failures are reported on
//!   stderr but do not change the exit code.
//! - **1** - The TUI itself failed (configuration load, terminal init, runtime
//!   error).
//!
//! ## Logging
//!
//! When the `logging` feature is enabled, errors are written to the log file
//! before being printed to stderr. See the `--log-file` argument for log file path.

/// Main entry point for the artifacts CLI.
///
/// Initializes the CLI, parses arguments, and runs the appropriate command.
/// On error, prints the error to stderr and exits with code 1.
#[tokio::main]
async fn main() {
    if let Err(err) = artifacts::cli::run().await {
        artifacts::log_error!("{:#}", err);
        eprintln!("{:#}", err);
        std::process::exit(1);
    }
}
