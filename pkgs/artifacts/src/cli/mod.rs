pub mod args;
mod logging;

use crate::config::backend::BackendConfiguration;
use crate::config::make::MakeConfiguration;
use crate::config::nix::build_make_from_flake;
use crate::tui::{
    TerminalGuard, build_filtered_model, install_panic_hook, run_async,
    validate_model_capabilities, TerminalEventSource,
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

pub async fn run() -> Result<()> {
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

    // Debug logging - use OpenOptions to append
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/artifacts_debug.log")
            .expect("Failed to open log file");
        writeln!(
            file,
            "[DEBUG] Starting run_tui with {} entries",
            model.entries.len()
        )
        .expect("Failed to write");
    }

    // Install panic hook to restore terminal on crash
    install_panic_hook();

    // Initialize terminal
    let mut terminal_guard = TerminalGuard::new().context("Failed to initialize terminal")?;

    std::fs::write(
        "/tmp/artifacts_debug.log",
        "[DEBUG] About to call run_async\n",
    )
    .ok();

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
