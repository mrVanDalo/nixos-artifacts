pub mod args;
pub mod commands;

use anyhow::Result;
use clap::Parser;
use log::{Level, LevelFilter, Metadata, Record};
use std::io::{self, Write};

struct StdSplitLogger;

impl log::Log for StdSplitLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        match record.level() {
            Level::Error => {
                let msg = record.args().to_string();
                for line in msg.lines() {
                    let _ = writeln!(io::stderr(), "ERROR: {}", line);
                }
            }
            Level::Warn => {
                let msg = record.args().to_string();
                for line in msg.lines() {
                    let _ = writeln!(io::stdout(), "WARNING: {}", line);
                }
            }
            Level::Debug => {
                let msg = record.args().to_string();
                for line in msg.lines() {
                    let _ = writeln!(io::stdout(), "DEBUG: {}", line);
                }
            }
            Level::Info => {
                let _ = writeln!(io::stdout(), "{}", record.args());
            }
            Level::Trace => {
                let msg = record.args().to_string();
                for line in msg.lines() {
                    let _ = writeln!(io::stdout(), "TRACE: {}", line);
                }
            }
        }
    }
    fn flush(&self) {}
}

static LOGGER: StdSplitLogger = StdSplitLogger;

fn init_logger() {
    // Set once; ignore error if already set
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Debug));
}

pub fn run() -> Result<()> {
    init_logger();
    let cli = args::Cli::parse();

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
            artifact,
        } => {
            use std::path::{Path, PathBuf};
            let backend_path = std::env::var("NIXOS_ARTIFACTS_BACKEND_CONFIG")
                .map(PathBuf::from)
                .map_err(|_| anyhow::anyhow!("environment variable NIXOS_ARTIFACTS_BACKEND_CONFIG must be set and point to backend.toml"))?;
            if !backend_path.is_file() {
                return Err(anyhow::anyhow!(
                    "NIXOS_ARTIFACTS_BACKEND_CONFIG points to a non-existent file: {}",
                    backend_path.display()
                ));
            }

            // Determine flake path: use provided path or default to current directory, then run nix build with inline expr
            let flake_path: PathBuf = if let Some(p) = make { p } else { std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")) };

            // Ensure nix is available
            let nix_bin = which::which("nix").map_err(|_| anyhow::anyhow!("'nix' command not found in PATH. Please install Nix."))?;
            let mut cmd = std::process::Command::new(nix_bin);
            let expr = r#"
let
  system = "x86_64-linux";
  filterAttrs =
    pred: set:
    builtins.removeAttrs set (builtins.filter (name: !pred name set.${name}) (builtins.attrNames set));
  flake = builtins.getFlake (toString <flake>);
  pkgs = flake.inputs.nixpkgs.legacyPackages.${system};
  configurations = builtins.attrNames (
    filterAttrs (
      machine: configuration: builtins.hasAttr "artifacts" configuration.options
    ) flake.nixosConfigurations
  );
  make = map (name: {
    machine = name;
    artifacts = flake.nixosConfigurations.${name}.config.artifacts.store;
    config =
      if (builtins.hasAttr "config" flake.nixosConfigurations.${name}.config.artifacts) then
        flake.nixosConfigurations.${name}.config.artifacts.config
      else
        { };
  }) configurations;
in
pkgs.writeText "test.json" (builtins.toJSON make)
"#;
            cmd.arg("build")
                .arg("--impure")
                .arg("-I")
                .arg(format!("flake={}", flake_path.display()))
                .arg("--no-link")
                .arg("--print-out-paths")
                .arg("--expr")
                .arg(expr);
            let output = cmd.output().map_err(|e| anyhow::anyhow!("failed to start nix build: {}", e))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                return Err(anyhow::anyhow!(
                    "nix build failed. stdout: {}\nstderr: {}",
                    stdout,
                    stderr
                ));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let path_line = stdout.lines().last().unwrap_or("").trim();
            if path_line.is_empty() {
                return Err(anyhow::anyhow!("nix build did not return a store path"));
            }
            let make_path = Path::new(path_line).to_path_buf();
            if !make_path.is_file() {
                return Err(anyhow::anyhow!(
                    "nix build returned a path that is not a file: {}",
                    make_path.display()
                ));
            }

            commands::generate::run_generate_command(&backend_path, &make_path, all, &machine, &artifact)?
        }
        args::Command::Regenerate {
            backend,
            make,
            all,
            machine,
            artifact,
        } => commands::generate::run_regenerate_command(&backend, &make, all, &machine, &artifact)?,
        args::Command::List { backend: _, make } => commands::list::run(&make)?,
    }
    Ok(())
}
