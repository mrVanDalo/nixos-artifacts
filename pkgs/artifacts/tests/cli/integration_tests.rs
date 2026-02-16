//! CLI integration tests using insta-cmd for snapshot testing.
//!
//! These tests verify CLI behavior including help output, version,
//! and error handling when run without a flake.

use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use serial_test::serial;
use std::process::Command;

/// Get the path to the artifacts binary.
fn cli() -> Command {
    Command::new(get_cargo_bin("artifacts"))
}

/// CLI test with no arguments should fail (requires flake or backend.toml).
#[test]
#[serial]
fn cli_no_args_shows_error() {
    assert_cmd_snapshot!(cli());
}

/// CLI help output.
#[test]
#[serial]
fn cli_help() {
    assert_cmd_snapshot!(cli().arg("--help"));
}

/// CLI version output.
#[test]
#[serial]
fn cli_version() {
    assert_cmd_snapshot!(cli().arg("--version"));
}

/// CLI with invalid flake path shows error.
#[test]
#[serial]
fn cli_invalid_flake_path() {
    assert_cmd_snapshot!(cli().arg("/nonexistent/path"));
}

/// CLI with --log-level argument.
#[test]
#[serial]
fn cli_with_log_level() {
    assert_cmd_snapshot!(cli().args(["--log-level", "debug"]));
}

/// CLI with --no-emoji flag.
#[test]
#[serial]
fn cli_with_no_emoji() {
    assert_cmd_snapshot!(cli().arg("--no-emoji"));
}

/// CLI with machine filter.
#[test]
#[serial]
fn cli_with_machine_filter() {
    assert_cmd_snapshot!(cli().args(["--machine", "test-machine"]));
}
