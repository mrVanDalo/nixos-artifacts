pub mod args;
pub mod commands;

use anyhow::Result;
use clap::{CommandFactory, Parser};

pub fn run() -> Result<()> {
    let cli = args::Cli::parse();
    match cli.command {
        args::Command::Generate { backend, make } => commands::generate::run(&backend, &make)?,
    }
    Ok(())
}
