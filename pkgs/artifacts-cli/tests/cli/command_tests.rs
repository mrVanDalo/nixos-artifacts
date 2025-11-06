use anyhow::Context;
use anyhow::Result;
#[allow(deprecated)]
use insta_cmd::{StdinCommand, assert_cmd_snapshot, get_cargo_bin};
use serial_test::serial;
use std::fs::remove_dir_all;
use std::path::{Path, PathBuf};
use std::process::Command;

fn cli() -> Command {
    Command::new(get_cargo_bin("artifacts"))
}

// todo get rid of /nix/store/<hash> in snapshots
// todo get rid of /tmp/<hash> in snapshots => get rid of serial

#[allow(deprecated)]
fn sdtin_cli(stdin: &str) -> StdinCommand {
    let mut cmd = StdinCommand::new(get_cargo_bin("artifacts"), stdin);
    // StdinCommand::env returns &mut Command; we don't need the return value here.
    let _ = cmd
        .env("TMPDIR", "/tmp/artifacts-cli")
        .arg("--log-level=trace");
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
        let path = PathBuf::from("/tmp/artifacts-cli");
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
fn no_config_scenario() {
    let root = project_root();
    let test_dir = root.join("examples/no_config");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("one\ntwo\n");

    cmd.arg("generate")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}
#[test]
#[serial]
fn scenario_simple() {
    let root = project_root();
    let test_dir = root.join("examples/scenario_simple");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("one\ntwo\n");

    cmd.arg("generate")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn two_artifacts_scenario() {
    let root = project_root();
    let test_dir = root.join("examples/2_artifacts");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("");

    cmd.arg("generate")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");
    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn missing_files_scenario() {
    let root = project_root();
    let test_dir = root.join("examples/missing-files");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("one\ntwo\n");

    cmd.arg("generate")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn wrong_file_type_scenario() {
    let root = project_root();
    let test_dir = root.join("examples/wrong-file-type");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("one\ntwo\n");

    cmd.arg("generate")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn unwanted_files_scenario() {
    let root = project_root();
    let test_dir = root.join("examples/unwanted-files");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("one\ntwo\n");

    cmd.arg("generate")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn missing_generator_scenario() {
    let root = project_root();
    let test_dir = root.join("examples/missing_generator");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("");

    cmd.arg("generate")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
fn scenario_help() {
    let mut cmd = cli();
    cmd.arg("--help");

    assert_cmd_snapshot!(cmd);
}

#[test]
fn scenario_generator_help() {
    let mut cmd = cli();
    cmd.arg("generate").arg("--help");

    assert_cmd_snapshot!(cmd);
}

#[test]
fn scenario_regenerator_help() {
    let mut cmd = cli();
    cmd.arg("regenerate").arg("--help");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn list_scenarios() {
    let root = project_root();
    let test_dir = root.join("examples/bigger_setup");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("");

    cmd.arg("list")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn bigger_scenarios() {
    let root = project_root();
    let test_dir = root.join("examples/bigger_setup");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("");

    cmd.arg("generate")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn regenerate_all_scenarios() {
    let root = project_root();
    let test_dir = root.join("examples/bigger_setup");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("");

    cmd.arg("regenerate")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir)
        .arg("--all");

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn regenerate_machine_scenarios() {
    let root = project_root();
    let test_dir = root.join("examples/bigger_setup");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("");

    cmd.arg("regenerate")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir)
        .arg("--machine=machine-one");

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn regenerate_machine_and_artifacts_scenarios() {
    let root = project_root();
    let test_dir = root.join("examples/bigger_setup");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("");

    cmd.arg("regenerate")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir)
        .arg("--machine=machine-one")
        .arg("--artifact=artifact-one");

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn regenerate_wrong_machine_scenarios() {
    let root = project_root();
    let test_dir = root.join("examples/bigger_setup");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("");

    cmd.arg("regenerate")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir)
        .arg("--machine=machine-name");

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

#[test]
#[serial]
fn artifact_name_scenario() {
    let root = project_root();
    let test_dir = root.join("examples/artifact_names");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("");

    cmd.arg("generate")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir)
        .arg("--machine=machine-name");

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}

//#[test]
//#[serial]
fn simple_home_manager_scenario() {
    let root = project_root();
    let test_dir = root.join("examples/simple-home-manager");

    let env = TempTestEnv::new();

    let mut cmd = sdtin_cli("");

    cmd.arg("generate")
        .env(
            "NIXOS_ARTIFACTS_BACKEND_CONFIG",
            &test_dir.join("backend.toml"),
        )
        .arg(test_dir);

    // Verify and cleanup
    env.finish().expect("temp folder not empty at end of test");

    assert_cmd_snapshot!(cmd);
}
