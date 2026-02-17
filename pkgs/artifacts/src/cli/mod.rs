pub mod args;
pub mod headless;

use crate::config::backend::BackendConfiguration;
use crate::config::make::MakeConfiguration;
use crate::config::nix::build_make_from_flake;
use crate::tui::{
    TerminalEventSource, TerminalGuard, build_filtered_model, install_panic_hook, run_async,
    validate_model_capabilities,
};
use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};

fn resolve_flake_path(p: &Option<PathBuf>) -> PathBuf {
    p.clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

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
    #[cfg(feature = "logging")]
    {
        use crate::logging;
        if let Err(error) = logging::init_from_args(&cli) {
            eprintln!("Failed to initialize logging: {}", error);
            // Continue anyway - logging is optional
        }
    }

    // Resolve paths
    let flake_path = resolve_flake_path(&cli.flake);
    let backend_path = resolve_backend_toml(&flake_path)?;
    let make_path = build_make_from_flake(&flake_path)?;

    // Run TUI
    run_tui(
        &backend_path,
        &make_path,
        &cli.machine,
        &cli.home,
        &cli.artifact,
    )
    .await
}

async fn run_tui(
    backend_path: &Path,
    make_path: &Path,
    machines: &[String],
    home_users: &[String],
    artifacts: &[String],
) -> Result<()> {
    // Load configurations
    let backend = BackendConfiguration::read_backend_config(backend_path)?;
    let make = MakeConfiguration::read_make_config(make_path)?;

    // Build the initial model
    let mut model = build_filtered_model(&make, machines, home_users, artifacts);

    // Validate backend capabilities and add warnings
    validate_model_capabilities(&mut model, &backend);

    if model.entries.is_empty() {
        println!("No artifacts found matching the specified filters.");
        return Ok(());
    }

    // Log TUI startup
    #[cfg(feature = "logging")]
    crate::info!("Starting run_tui with {} entries", model.entries.len());

    // Install panic hook to restore terminal on crash
    install_panic_hook();

    // Initialize terminal
    let mut terminal_guard = TerminalGuard::new().context("Failed to initialize terminal")?;

    // Log before running async TUI
    #[cfg(feature = "logging")]
    crate::info!("About to call run_async");

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
                .artifacts
                .iter()
                .filter_map(|a| match &a.status {
                    crate::app::model::ArtifactStatus::Failed { error, .. } => {
                        Some(format!("{}/{}: {}", a.target, a.artifact.name, error))
                    }
                    _ => None,
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
