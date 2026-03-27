//! Command-line interface for the artifacts CLI.
//!
//! This module handles CLI argument parsing, path resolution, and TUI initialization.
//! It serves as the bridge between the command-line interface and the interactive
//! TUI application.
//!
//! # CLI Flow
//!
//! 1. Parse command-line arguments using [`clap`]
//! 2. Resolve flake directory path (default: current directory)
//! 3. Resolve backend.toml path (env var or flake directory)
//! 4. Build make configuration from Nix flake evaluation
//! 5. Initialize terminal and run TUI
//!
//! # Environment Variables
//!
//! - `NIXOS_ARTIFACTS_BACKEND_CONFIG` - Override path to backend.toml
//!
//! # Exit Codes
//!
//! - `0` - Success (all artifacts processed)
//! - `1` - Error (failed artifacts or configuration error)

pub mod args;
pub mod headless;

use crate::config::backend::BackendConfiguration;
use crate::config::make::MakeConfiguration;
use crate::config::nix::build_make_from_flake;
use crate::log_info;
use crate::tui::{
    TerminalEventSource, TerminalGuard, build_model, install_panic_hook, run_async,
    validate_model_capabilities,
};
use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};

/// Resolve the flake directory path.
///
/// Returns the provided path if given, otherwise attempts to use
/// the current working directory, falling back to "." on error.
///
/// # Arguments
///
/// * `p` - Optional path from CLI arguments
fn resolve_flake_path(p: &Option<PathBuf>) -> PathBuf {
    p.clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// Resolve the backend.toml configuration file path.
///
/// Checks in order:
/// 1. `NIXOS_ARTIFACTS_BACKEND_CONFIG` environment variable
/// 2. `{flake_path}/backend.toml`
///
/// # Arguments
///
/// * `flake_path` - Path to the flake directory
///
/// # Errors
///
/// Returns an error if:
/// - The env var is set but points to a non-existent file
/// - No backend.toml exists in the flake directory
fn resolve_backend_toml(flake_path: &Path) -> Result<PathBuf> {
    match std::env::var("NIXOS_ARTIFACTS_BACKEND_CONFIG") {
        Ok(val) => {
            let p = PathBuf::from(val);
            if !p.is_file() {
                Err(anyhow::anyhow!(
                    "NIXOS_ARTIFACTS_BACKEND_CONFIG points to a non-existent file: {}",
                    p.display()
                ))
            } else {
                Ok(p)
            }
        }
        Err(_) => {
            let p = flake_path.join("backend.toml");
            if !p.is_file() {
                Err(anyhow::anyhow!(
                    "backend.toml not found. Set NIXOS_ARTIFACTS_BACKEND_CONFIG or place backend.toml in the flake directory: {}",
                    p.display()
                ))
            } else {
                Ok(p)
            }
        }
    }
}

pub async fn run() -> Result<()> {
    let cli = args::Cli::parse();

    // Initialize logger first using new macro-based system
    {
        use crate::logging;
        use crate::logging::LogLevel;
        let log_file = cli.log_file.as_deref();
        let log_level = LogLevel::from_cli_level(&cli.log_level);
        if let Err(error) = logging::init(log_file, log_level) {
            eprintln!("Failed to initialize logging: {}", error);
            // Continue anyway - logging is optional
        }
    }

    // Resolve paths
    let flake_path = resolve_flake_path(&cli.flake);
    let backend_path = resolve_backend_toml(&flake_path)?;
    let make_path = build_make_from_flake(&flake_path)?;

    // Run TUI
    run_tui(&backend_path, &make_path).await
}

async fn run_tui(backend_path: &Path, make_path: &Path) -> Result<()> {
    // STEP 1: Load all configurations BEFORE terminal setup (ERR-01)
    // Errors here print to stderr and exit non-zero
    let backend = BackendConfiguration::read_backend_config(backend_path).with_context(|| {
        format!(
            "Failed to load backend configuration from '{}'",
            backend_path.display()
        )
    })?;

    let make = MakeConfiguration::read_make_config(make_path)
        .with_context(|| "Failed to load artifact definitions from nix evaluation".to_string())?;

    // Build the initial model
    let mut model = build_model(&make);

    // Validate backend capabilities and add warnings
    validate_model_capabilities(&mut model, &backend);

    // STEP 4: Check for empty entries BEFORE terminal setup (UI-03)
    if model.entries.is_empty() {
        log_info!("No artifacts found");
        println!("No artifacts found.");
        return Ok(());
    }

    // STEP 5: Install panic hook BEFORE terminal (ERR-04)
    install_panic_hook();

    // STEP 6: Initialize terminal
    let mut terminal_guard = TerminalGuard::new().context("Failed to initialize terminal")?;

    // Log TUI startup with entry count
    log_info!("Starting run_tui with {} entries", model.entries.len());

    log_info!("About to call run_async");

    // Run the TUI asynchronously with terminal event source
    let mut events = TerminalEventSource::default();
    let result = run_async(terminal_guard.terminal(), &mut events, backend, make, model).await;

    // Restore terminal before handling result
    terminal_guard
        .restore()
        .context("Failed to restore terminal")?;

    match result {
        Ok(run_result) => {
            let failed: Vec<_> = run_result
                .final_model
                .entries
                .iter()
                .filter_map(|entry| match entry {
                    crate::app::model::ListEntry::Single(a) => match &a.status {
                        crate::app::model::ArtifactStatus::Failed { error, .. } => {
                            let target = a.target_type.target_name().unwrap_or("unknown");
                            Some(format!("{}/{}: {}", target, a.artifact.name, error))
                        }
                        _ => None,
                    },
                    crate::app::model::ListEntry::Shared(s) => match &s.status {
                        crate::app::model::ArtifactStatus::Failed { error, .. } => {
                            Some(format!("shared/{}: {}", s.info.artifact_name, error))
                        }
                        _ => None,
                    },
                })
                .collect();

            if !failed.is_empty() {
                eprintln!("Failed artifacts:");
                for msg in &failed {
                    eprintln!("  {}", msg);
                }
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}
