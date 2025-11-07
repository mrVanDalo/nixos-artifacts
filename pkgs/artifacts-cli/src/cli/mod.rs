pub mod args;
pub mod commands;
mod logging;

use crate::config::nix::build_make_from_flake;
use anyhow::Result;
use clap::Parser;
use log::LevelFilter;

fn resolve_flake_path(p: &Option<std::path::PathBuf>) -> std::path::PathBuf {
    p.clone().unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    })
}

fn resolve_backend_toml(flake_path: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
    use std::path::PathBuf;
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

    // Initialize logger based on emoji preference
    logging::init_logger(!cli.no_emoji);

    // Configure log level based on CLI argument (default is Debug from init, but override if provided)
    let level_filter = match cli.log_level {
        args::LogLevel::Error => LevelFilter::Error,
        args::LogLevel::Warning => LevelFilter::Warn,
        args::LogLevel::Info => LevelFilter::Info,
        args::LogLevel::Debug => LevelFilter::Debug,
        args::LogLevel::Trace => LevelFilter::Trace,
    };
    log::set_max_level(level_filter);

    match cli.command {
        args::Command::Generate {
            make,
            all,
            machine,
            home,
            artifact,
        } => {
            let flake_path = resolve_flake_path(&make);
            let backend_path = resolve_backend_toml(&flake_path)?;
            let make_path = build_make_from_flake(&flake_path)?;

            commands::generate::run_generate_command(
                &backend_path,
                &make_path,
                all,
                &machine,
                &home,
                &artifact,
            )?
        }
        args::Command::Regenerate {
            make,
            all,
            machine,
            home,
            artifact,
        } => {
            let flake_path = resolve_flake_path(&make);
            let backend_path = resolve_backend_toml(&flake_path)?;
            let make_path = build_make_from_flake(&flake_path)?;
            commands::generate::run_regenerate_command(
                &backend_path,
                &make_path,
                all,
                &machine,
                &home,
                &artifact,
            )?
        }
        args::Command::List { make } => {
            let flake_path = resolve_flake_path(&make);
            // Resolve backend too, to meet the requirement that all commands read it.
            let _backend_path = resolve_backend_toml(&flake_path)?;
            let make_path = build_make_from_flake(&flake_path)?;
            commands::list::run(&make_path)?
        }
    }
    Ok(())
}
