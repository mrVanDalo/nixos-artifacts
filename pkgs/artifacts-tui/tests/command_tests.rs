use anyhow::Context;
use anyhow::Result;
#[allow(deprecated)]
use insta_cmd::{StdinCommand, assert_cmd_snapshot, get_cargo_bin};
use serial_test::serial;
use std::fs::remove_dir_all;
use std::path::{Path, PathBuf};
use std::process::Command;

fn cli() -> Command {
    Command::new(get_cargo_bin("artifacts-cli"))
}

#[allow(deprecated)]
fn sdtin_cli(stdin: &str) -> StdinCommand {
    let mut cmd = StdinCommand::new(get_cargo_bin("artifacts-cli"), stdin);
    // StdinCommand::env returns &mut Command; we don't need the return value here.
    let _ = cmd
        .env("TMPDIR", "/tmp/artifacts-tui-ci")
        .arg("--log-level=debug");
    // let _ = cmd.env("ARTIFACTS_TUI_TEST_FIXED_TMP", "1");
    cmd
}

fn project_root() -> PathBuf {
    // tests run with CWD at crate root, but compute robustly
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Helper to manage a fixed temp directory for deterministic snapshots.
/// It ensures the directory exists before a test, and after the test
/// verifies it is empty; if not, the test fails, but the directory is
/// deleted in any case.
struct TempTestEnv {
    path: PathBuf,
}

impl Drop for TempTestEnv {
    fn drop(&mut self) {
        // Best-effort cleanup; ignore errors so Drop never panics
        let _ = remove_dir_all(&self.path);
    }
}

#[allow(deprecated)]
impl TempTestEnv {
    fn new() -> Self {
        let path = PathBuf::from("/tmp/artifacts-tui-ci");
        let _ = remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("failed to create fixed tmp dir");
        TempTestEnv { path }
    }

    fn is_empty_dir(path: &Path) -> std::io::Result<bool> {
        let mut entries = std::fs::read_dir(path)?;
        Ok(entries.next().is_none())
    }

    fn finish(self) -> Result<()> {
        Self::is_empty_dir(&self.path)
            .context("failed to read tmp dir")?
            .then_some(())
            .ok_or_else(|| anyhow::anyhow!("temporary directory not empty before cleanup"))?;
        Ok(())
    }
}

#[test]
#[serial]
fn scenario_simple() {
    let root = project_root();
    let backend = root.join("examples/scenario_simple/backend.toml");
    let make = root.join("examples/scenario_simple/make.json");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("one\ntwo\n");

    cmd.arg("generate").arg(backend).arg(make);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn generator_missing_scenario() {
    let root = project_root();
    let backend = root.join("examples/generator_missing/backend.toml");
    let make = root.join("examples/generator_missing/make.json");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("one\ntwo\n");

    cmd.arg("generate").arg(backend).arg(make);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");
    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn two_artifacts_scenario() {
    let root = project_root();
    let backend = root.join("examples/2_artifacts/backend.toml");
    let make = root.join("examples/2_artifacts/make.json");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("one\ntwo\n");

    cmd.arg("generate").arg(backend).arg(make);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");
    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn generate_wrong_missing_files_scenario() {
    let root = project_root();
    let backend = root.join("examples/generate_wrong/backend.toml");
    let make = root.join("examples/generate_wrong/make_missing_files.json");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("one\ntwo\n");

    cmd.arg("generate").arg(backend).arg(make);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn generate_wrong_wrong_file_type_scenario() {
    let root = project_root();
    let backend = root.join("examples/generate_wrong/backend.toml");
    let make = root.join("examples/generate_wrong/make_wrong_file_type.json");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("one\ntwo\n");

    cmd.arg("generate").arg(backend).arg(make);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn generate_wrong_unwanted_files_scenario() {
    let root = project_root();
    let backend = root.join("examples/generate_wrong/backend.toml");
    let make = root.join("examples/generate_wrong/make_unwanted_files.json");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("one\ntwo\n");

    cmd.arg("generate").arg(backend).arg(make);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn missing_generator_scenario() {
    let root = project_root();
    let backend = root.join("examples/missing_generator/backend.toml");
    let make = root.join("examples/missing_generator/make.json");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("");

    cmd.arg("generate").arg(backend).arg(make);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn scenario_help() {
    let mut cmd = cli();
    cmd.arg("generate").arg("--help");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn list_scenarios() {
    let root = project_root();
    let backend = root.join("examples/2_artifacts/backend.toml");
    let make = root.join("examples/2_artifacts/make.json");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("");

    cmd.arg("list").arg(backend).arg(make);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}
