pub mod args;
mod logging;

use crate::config::backend::BackendConfiguration;
use crate::config::make::MakeConfiguration;
use crate::config::nix::build_make_from_flake;
use crate::tui::{
    build_filtered_model, install_panic_hook, run as run_tui_loop, BackendEffectHandler,
    TerminalEventSource, TerminalGuard,
};
use anyhow::{Context, Result};
use clap::Parser;
use log::LevelFilter;
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

pub fn run() -> Result<()> {
    let cli = args::Cli::parse();

    // Initialize logger
    logging::init_logger(!cli.no_emoji);

    let level_filter = match cli.log_level {
        args::LogLevel::Error => LevelFilter::Error,
        args::LogLevel::Warning => LevelFilter::Warn,
        args::LogLevel::Info => LevelFilter::Info,
        args::LogLevel::Debug => LevelFilter::Debug,
        args::LogLevel::Trace => LevelFilter::Trace,
    };
    log::set_max_level(level_filter);

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
}

fn run_tui(
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
    let model = build_filtered_model(&make, machines, home_users, artifacts);

    if model.artifacts.is_empty() {
        println!("No artifacts found matching the specified filters.");
        return Ok(());
    }

    // Install panic hook to restore terminal on crash
    install_panic_hook();

    // Initialize terminal
    let mut terminal_guard = TerminalGuard::new().context("Failed to initialize terminal")?;

    // Create event source and effect handler
    let mut events = TerminalEventSource::default();
    let mut effects = BackendEffectHandler::new(backend, make);

    // Run the TUI
    let result = run_tui_loop(
        terminal_guard.terminal(),
        &mut events,
        &mut effects,
        model,
    );

    // Restore terminal before handling result
    terminal_guard
        .restore()
        .context("Failed to restore terminal")?;

    match result {
        Ok(run_result) => {
            let generated = run_result
                .final_model
                .artifacts
                .iter()
                .filter(|a| matches!(a.status, crate::app::model::ArtifactStatus::Done))
                .count();
            let failed = run_result
                .final_model
                .artifacts
                .iter()
                .filter(|a| matches!(a.status, crate::app::model::ArtifactStatus::Failed(_)))
                .count();

            if generated > 0 || failed > 0 {
                println!("Generated: {}, Failed: {}", generated, failed);
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}
