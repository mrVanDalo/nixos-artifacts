use insta_cmd::{Spawn, SpawnExt, assert_cmd_snapshot, get_cargo_bin, write_stdin};
use std::path::PathBuf;
use std::process::Command;

fn cli() -> Command {
    Command::new(get_cargo_bin("artifacts-cli"))
}

fn project_root() -> PathBuf {
    // tests run with CWD at crate root, but compute robustly
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn scenario_simple() {
    let root = project_root();
    let backend = root.join("examples/scenario_simple/backend.toml");
    let make = root.join("examples/scenario_simple/make.json");

    // Stabilize temp directory paths for deterministic snapshots
    let fixed_tmp = PathBuf::from("/tmp/artifacts-tui-ci");
    let _ = std::fs::remove_dir_all(&fixed_tmp);
    std::fs::create_dir_all(&fixed_tmp).unwrap();

    let mut cmd = cli();

    cmd.env("TMPDIR", &fixed_tmp)
        .env("ARTIFACTS_TUI_TEST_FIXED_TMP", "1")
        .arg("generate")
        .arg(backend)
        .arg(make)
        .pass_stdin("test");

    assert_cmd_snapshot!(cmd);
}

#[test]
fn scenario_help() {
    let mut cmd = cli();
    cmd.arg("generate").arg("--help");

    assert_cmd_snapshot!(cmd);
}
