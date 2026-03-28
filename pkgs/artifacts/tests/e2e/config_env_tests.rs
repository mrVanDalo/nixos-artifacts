//! Tests to verify that backend scripts receive correct environment variables.
//!
//! These tests verify that:
//! - `$config` is set for NixOS and Home Manager targets
//! - `$config` file exists and contains valid JSON
//! - `$machines` and `$users` are set for shared artifacts (not `$config`)
//! - `$machines` and `$users` files exist and contain valid JSON
//!
//! The tests use the test-config-verify backend which outputs the environment
//! variable contents for snapshot verification.
//!
//! How to run:
//! - All e2e tests: cargo test --test tests e2e
//! - These tests: cargo test --test tests e2e_config
//! - Specific test: cargo test --test tests e2e_config_nixos_check_sets_config

use crate::common::{CleanupGuard, setup_test_storage};
use anyhow::Result;
use artifacts::app::model::TargetType;
use artifacts::backend::serialization::{
    run_check_serialization, run_serialize, run_shared_check_serialization, run_shared_serialize,
};
use artifacts::config::backend::BackendConfiguration;
use artifacts::config::nix::build_make_from_flake;
use serial_test::serial;
use std::path::PathBuf;
use tempfile::TempDir;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_example(
    name: &str,
) -> Result<(
    BackendConfiguration,
    artifacts::config::make::MakeConfiguration,
)> {
    let example_dir = project_root().join("examples").join(name);

    let backend = BackendConfiguration::read_backend_config(&example_dir.join("backend.toml"))?;

    let make_path = build_make_from_flake(&example_dir)?;
    let make = artifacts::config::make::MakeConfiguration::read_make_config(&make_path)?;

    Ok((backend, make))
}

fn find_first_nixos_artifact(
    make_config: &artifacts::config::make::MakeConfiguration,
    machine_name: &str,
) -> Option<(String, artifacts::config::make::ArtifactDef)> {
    make_config
        .nixos_map
        .get(machine_name)
        .and_then(|artifacts| {
            artifacts
                .iter()
                .next()
                .map(|(name, def)| (name.clone(), def.clone()))
        })
}

fn find_shared_artifact(
    make_config: &artifacts::config::make::MakeConfiguration,
    artifact_name: &str,
) -> Option<(String, artifacts::config::make::ArtifactDef)> {
    for (_machine, artifacts) in &make_config.nixos_map {
        for (name, def) in artifacts {
            if name == artifact_name && def.shared {
                let mut def_clone = def.clone();
                def_clone.name = name.clone();
                return Some((name.clone(), def_clone));
            }
        }
    }
    None
}

/// Assert snapshot with temp paths redacted.
/// Uses insta filters to replace dynamic /tmp/ paths with /tmp/REDACTED.
macro_rules! assert_snapshot_redacted {
    ($output:expr) => {
        insta::with_settings!({filters => vec![
            (r"/tmp/[\w.-]+", "/tmp/REDACTED"),
        ]}, {
            insta::assert_snapshot!($output);
        });
    };
}

// =============================================================================
// TEST: check_serialization sets $config for NixOS targets
// =============================================================================

/// Verify that run_check_serialization sets $config correctly for NixOS artifacts.
/// Snapshot shows the full output including config path, machine, artifact, and JSON content.
#[test]
#[serial]
fn e2e_config_nixos_check_sets_config() -> Result<()> {
    let _cleanup = CleanupGuard;
    let (backend, make_config) = load_example("scenarios/config-verify")?;

    let (_artifact_name, artifact_def) = find_first_nixos_artifact(&make_config, "test-machine")
        .ok_or_else(|| anyhow::anyhow!("No NixOS artifact found"))?;

    let result = run_check_serialization(
        &artifact_def,
        &TargetType::NixOS {
            machine: "test-machine".to_string(),
        },
        &backend,
        &make_config,
        "info",
    )?;

    assert_snapshot_redacted!(result.output.to_string());

    Ok(())
}

// =============================================================================
// TEST: serialize sets $config for NixOS targets
// =============================================================================

/// Verify that run_serialize sets $config correctly for NixOS artifacts.
/// Snapshot shows the full output including config path, machine, artifact, and JSON content.
#[test]
#[serial]
fn e2e_config_nixos_serialize_sets_config() -> Result<()> {
    let (_temp_dir, _storage_path) = setup_test_storage()?;
    let _cleanup = CleanupGuard;
    let (backend, make_config) = load_example("scenarios/config-verify")?;

    let (_artifact_name, artifact_def) = find_first_nixos_artifact(&make_config, "test-machine")
        .ok_or_else(|| anyhow::anyhow!("No NixOS artifact found"))?;

    // Create temp output directory with generated files
    let out_dir = TempDir::new()?;
    std::fs::write(out_dir.path().join("secret-file"), "test-secret-content")?;

    let result = run_serialize(
        &artifact_def,
        &backend,
        out_dir.path(),
        &TargetType::NixOS {
            machine: "test-machine".to_string(),
        },
        &make_config,
        "info",
    )?;

    assert_snapshot_redacted!(result.to_string());

    Ok(())
}

// =============================================================================
// TEST: shared_check_serialization sets $machines and $users
// =============================================================================

/// Verify that run_shared_check_serialization sets $machines and $users correctly.
/// Snapshot shows the full output including machines/users JSON content.
#[test]
#[serial]
fn e2e_config_shared_check_sets_machines_users() -> Result<()> {
    let _cleanup = CleanupGuard;
    let (backend, make_config) = load_example("scenarios/config-verify")?;

    let (artifact_name, artifact_def) = find_shared_artifact(&make_config, "shared-config-secret")
        .ok_or_else(|| anyhow::anyhow!("No shared artifact found"))?;

    // Get all machines that have this shared artifact
    let nixos_targets: Vec<String> = make_config.nixos_map.keys().cloned().collect();

    let result = run_shared_check_serialization(
        &artifact_name,
        &artifact_def.serialization,
        &backend,
        &make_config,
        &nixos_targets,
        &[], // No home targets in this test
        "info",
    )?;

    assert_snapshot_redacted!(result.output.to_string());

    Ok(())
}

// =============================================================================
// TEST: shared_serialize sets $machines and $users
// =============================================================================

/// Verify that run_shared_serialize sets $machines and $users correctly.
/// Snapshot shows the full output including machines/users JSON content.
#[test]
#[serial]
fn e2e_config_shared_serialize_sets_machines_users() -> Result<()> {
    let (_temp_dir, _storage_path) = setup_test_storage()?;
    let _cleanup = CleanupGuard;
    let (backend, make_config) = load_example("scenarios/config-verify")?;

    let (artifact_name, artifact_def) = find_shared_artifact(&make_config, "shared-config-secret")
        .ok_or_else(|| anyhow::anyhow!("No shared artifact found"))?;

    // Get all machines that have this shared artifact
    let nixos_targets: Vec<String> = make_config.nixos_map.keys().cloned().collect();

    // Create temp output directory with generated files
    let out_dir = TempDir::new()?;
    std::fs::write(out_dir.path().join("shared-file"), "shared-content")?;

    let result = run_shared_serialize(
        &artifact_name,
        &artifact_def.serialization,
        &backend,
        out_dir.path(),
        &make_config,
        &nixos_targets,
        &[], // No home targets in this test
        "info",
    )?;

    assert_snapshot_redacted!(result.to_string());

    Ok(())
}
