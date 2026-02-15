use log::{Level, LevelFilter, Metadata, Record};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::sync::OnceLock;

/// The log file path for debug output.
const LOG_FILE_PATH: &str = "/tmp/artifacts_cli.log";

/// Global log file handle protected by a mutex for thread safety.
static LOG_FILE: OnceLock<Mutex<std::fs::File>> = OnceLock::new();

/// Initialize the log file handle.
fn init_log_file() -> Mutex<std::fs::File> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .truncate(false)
        .open(LOG_FILE_PATH)
        .expect("Failed to open log file");
    Mutex::new(file)
}

/// Write a message to the log file.
fn write_to_log(level: &str, msg: &str) {
    let file = LOG_FILE.get_or_init(init_log_file);
    if let Ok(mut guard) = file.lock() {
        let _ = writeln!(guard, "[{}] {}", level, msg);
        let _ = guard.flush();
    }
}

struct NoEmojiLogger;

impl log::Log for NoEmojiLogger {
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
                    write_to_log("ERROR", line);
                }
            }
            Level::Warn => {
                let msg = record.args().to_string();
                for line in msg.lines() {
                    write_to_log("WARN", line);
                }
            }
            Level::Debug => {
                let msg = record.args().to_string();
                for line in msg.lines() {
                    write_to_log("DEBUG", line);
                }
            }
            Level::Info => {
                write_to_log("INFO", &record.args().to_string());
            }
            Level::Trace => {
                let msg = record.args().to_string();
                for line in msg.lines() {
                    write_to_log("TRACE", line);
                }
            }
        }
    }
    fn flush(&self) {}
}

static LOGGER: NoEmojiLogger = NoEmojiLogger;

struct EmojiLogger;

impl log::Log for EmojiLogger {
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
                    write_to_log("ERROR", &format!("❌ {}", line));
                }
            }
            Level::Warn => {
                let msg = record.args().to_string();
                for line in msg.lines() {
                    write_to_log("WARN", &format!("⚠️ {}", line));
                }
            }
            Level::Debug => {
                let msg = record.args().to_string();
                for line in msg.lines() {
                    write_to_log("DEBUG", &format!("🐛 {}", line));
                }
            }
            Level::Info => {
                write_to_log("INFO", &record.args().to_string());
            }
            Level::Trace => {
                let msg = record.args().to_string();
                for line in msg.lines() {
                    write_to_log("TRACE", &format!("💬 {}", line));
                }
            }
        }
    }
    fn flush(&self) {}
}

static EMOJI_LOGGER: EmojiLogger = EmojiLogger;

pub fn init_logger(use_emojis: bool) {
    // Set once; ignore error if already set
    if use_emojis {
        let _ = log::set_logger(&EMOJI_LOGGER).map(|()| log::set_max_level(LevelFilter::Debug));
    } else {
        let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Debug));
    }
}
