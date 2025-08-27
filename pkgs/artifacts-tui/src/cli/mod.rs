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
    match cli.command {
        args::Command::Generate { backend, make } => commands::generate::run(&backend, &make)?,
    }
    Ok(())
}
