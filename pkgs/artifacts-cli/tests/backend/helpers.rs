use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use artifacts_cli::backend::helpers::{
    escape_single_quoted, fnv1a64, pretty_print_shell_escape, print_files,
};
use artifacts_cli::config::make::{ArtifactDef, FileDef};
use insta::{assert_debug_snapshot, assert_snapshot};
use log::{Level, LevelFilter, Log, Metadata, Record};

// // ---- Test Logger to capture `log::debug!` output from print_files ----
struct TestLogger {
    buf: Mutex<Vec<String>>, // collected log lines
}

impl TestLogger {
    fn new() -> Self {
        Self {
            buf: Mutex::new(Vec::new()),
        }
    }

    fn clear(&self) {
        self.buf.lock().unwrap().clear();
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.buf.lock().unwrap())
    }
}

impl Log for TestLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            self.buf.lock().unwrap().push(format!("{}", record.args()));
        }
    }

    fn flush(&self) {}
}

static LOGGER: OnceLock<TestLogger> = OnceLock::new();
static LOGGER_INIT_GUARD: OnceLock<()> = OnceLock::new();

fn init_test_logger() -> &'static TestLogger {
    LOGGER_INIT_GUARD.get_or_init(|| {
        let logger = LOGGER.get_or_init(TestLogger::new);
        let _ = log::set_logger(logger); // ignore error if already set
        log::set_max_level(LevelFilter::Debug);
    });
    LOGGER.get().unwrap()
}

// ---- Tests ----

#[test]
fn test_escape_single_quoted() {
    let cases = vec!["", "noquotes", "it's fine", "''''", "a'b"];
    let outputs: Vec<(String, String)> = cases
        .into_iter()
        .map(|inp| (inp.to_string(), escape_single_quoted(inp)))
        .collect();
    assert_debug_snapshot!("escape_single_quoted", outputs);
}

#[test]
fn test_pretty_print_shell_escape() {
    let cases = vec![
        "",
        "simple",
        "with space",
        "needs$var",
        "it's quoted",
        "already\"quoted\"",
        "path/with[brackets]",
        "no_specials",
    ];
    let outputs: Vec<(String, String)> = cases
        .into_iter()
        .map(|inp| (inp.to_string(), pretty_print_shell_escape(inp)))
        .collect();
    assert_debug_snapshot!("pretty_print_shell_escape", outputs);
}

#[test]
fn test_print_files_logs() {
    let logger = init_test_logger();
    logger.clear();

    // Build an artifact with a couple of files
    let mut files: BTreeMap<String, FileDef> = BTreeMap::new();
    files.insert(
        "config.toml".to_string(),
        FileDef {
            name: "config.toml".to_string(),
            path: Some("etc/app/config.toml".to_string()),
            owner: Some("root".to_string()),
            group: Some("root".to_string()),
        },
    );
    files.insert(
        "secret.key".to_string(),
        FileDef {
            name: "secret.key".to_string(),
            path: Some("/var/lib/app/secret.key".to_string()),
            owner: None,
            group: Some("app".to_string()),
        },
    );

    let artifact = ArtifactDef {
        name: "demo".to_string(),
        shared: false,
        files,
        prompts: BTreeMap::new(),
        generator: "gen.sh".to_string(),
        serialization: "ser.sh".to_string(),
    };

    let make_base = PathBuf::from("/make/base");

    print_files(&artifact, &make_base);

    let lines = logger.take().join("\n");
    assert_snapshot!("print_files_logs", lines);
}

#[test]
fn test_fnv1a64() {
    let cases = vec![
        "",
        "a",
        "hello",
        "Hello, world!",
        "/abs/path",
        "rel/path",
        "with spaces",
        "emoji 😀",
        "mix/With-CHARS_123",
    ];

    // Use hex to make the snapshot compact and stable across platforms
    let outputs: Vec<(String, String)> = cases
        .into_iter()
        .map(|inp| (inp.to_string(), format!("{:016x}", fnv1a64(inp))))
        .collect();

    insta::assert_debug_snapshot!("fnv1a64", outputs);
}
