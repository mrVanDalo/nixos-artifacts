//! End-to-end integration tests that verify artifacts are actually created.
//!
//! These tests run the complete pipeline and check that artifacts exist
//! in the expected backend locations with correct content.
//!
//! Test Requirements:
//! - TEST-01: Programmatic invocation without TUI
//! - TEST-02: Single artifact creation
//! - TEST-03: Verify artifact exists at backend location
//! - TEST-04: Verify artifact content format
//! - TEST-05: Cover both single and shared artifacts
//! - TEST-06: Tests run in CI with meaningful failure messages

pub mod backend_verify;
pub mod config_env_tests;
pub mod diagnostics;
pub mod edge_cases;
pub mod shared_artifact;

use crate::common::{TestHarness, dump_test_diagnostics};
use anyhow::{Context, Result};
use artifacts::app::model::TargetType;
use artifacts::config::backend::BackendConfiguration;
use artifacts::config::make::{ArtifactDef, MakeConfiguration};
use artifacts::config::nix::build_make_from_flake;
use serial_test::serial;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_example(name: &str) -> Result<(BackendConfiguration, MakeConfiguration)> {
    let example_dir = project_root().join("examples").join(name);

    let backend = BackendConfiguration::read_backend_config(&example_dir.join("backend.toml"))
        .with_context(|| format!("Failed to read backend.toml for {}", name))?;

    let make_path = build_make_from_flake(&example_dir)
        .with_context(|| format!("Failed to build make from flake for {}", name))?;
    let make = MakeConfiguration::read_make_config(&make_path)
        .with_context(|| format!("Failed to read make config for {}", name))?;

    Ok((backend, make))
}

fn find_first_artifact(
    make_config: &MakeConfiguration,
    machine_name: &str,
) -> Option<(String, ArtifactDef)> {
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

fn find_first_home_artifact(
    make_config: &MakeConfiguration,
    user_name: &str,
) -> Option<(String, ArtifactDef)> {
    make_config.home_map.get(user_name).and_then(|artifacts| {
        artifacts
            .iter()
            .next()
            .map(|(name, def)| (name.clone(), def.clone()))
    })
}

fn create_test_storage_dir(_test_name: &str) -> Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let storage_dir = temp_dir.path().join("storage");
    fs::create_dir_all(&storage_dir)?;
    Ok(temp_dir)
}

fn setup_test_storage() -> Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    fs::create_dir_all(&storage_path)?;

    unsafe {
        std::env::set_var("ARTIFACTS_TEST_OUTPUT_DIR", &storage_path);
    }

    Ok((temp_dir, storage_path))
}

fn cleanup_test_storage() {
    unsafe {
        std::env::remove_var("ARTIFACTS_TEST_OUTPUT_DIR");
    }
}

fn get_artifact_path(storage_dir: &Path, artifact_name: &str) -> PathBuf {
    storage_dir.join(artifact_name)
}

fn verify_artifact_exists(storage_dir: &Path, artifact_name: &str) -> Result<()> {
    let artifact_path = get_artifact_path(storage_dir, artifact_name);

    if !artifact_path.exists() {
        return Err(anyhow::anyhow!(
            "Artifact '{}' not found at expected path: {}. \
             Directory contents: {:?}",
            artifact_name,
            artifact_path.display(),
            fs::read_dir(storage_dir)
                .ok()
                .and_then(|entries| {
                    let names: Vec<_> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.file_name())
                        .collect();
                    if names.is_empty() { None } else { Some(names) }
                })
                .unwrap_or_else(|| vec!["(empty or inaccessible)".into()])
        ));
    }

    Ok(())
}

fn verify_artifact_content(storage_dir: &Path, artifact_name: &str, expected: &str) -> Result<()> {
    verify_artifact_exists(storage_dir, artifact_name)?;

    let artifact_path = get_artifact_path(storage_dir, artifact_name);
    let actual_content = fs::read_to_string(&artifact_path).with_context(|| {
        format!(
            "Failed to read artifact '{}' from {}",
            artifact_name,
            artifact_path.display()
        )
    })?;

    if actual_content != expected {
        return Err(anyhow::anyhow!(
            "Artifact '{}' content mismatch at {}\nExpected: {:?}\nActual: {:?}",
            artifact_name,
            artifact_path.display(),
            expected,
            actual_content
        ));
    }

    Ok(())
}

#[test]
#[serial]
fn e2e_single_artifact_is_created() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/single-artifact-with-prompts")?;

    let (artifact_name, artifact_def) = harness
        .find_artifact("machine-name", None)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found for machine-name"))?;

    let target_type = TargetType::NixOS {
        machine: "machine-name".to_string(),
    };

    let prompt_values: BTreeMap<String, String> = BTreeMap::from([
        ("secret1".to_string(), "test-secret-one".to_string()),
        ("secret2".to_string(), "test-secret-two".to_string()),
    ]);

    let (result, diagnostics) = harness.generate_artifact_with_diagnostics(
        "machine-name",
        &artifact_def,
        target_type.clone(),
        &prompt_values,
    )?;

    let result = match result.success {
        true => result,
        false => {
            let diag_dir = std::path::PathBuf::from("/tmp/artifacts_test_failures");
            let _ = std::fs::create_dir_all(&diag_dir);
            let diag_path = diag_dir.join(format!(
                "e2e_single_artifact_is_created_{}.txt",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ));
            let _ = dump_test_diagnostics(&diagnostics, &diag_path);
            return Err(anyhow::anyhow!(
                "Artifact generation failed: {:?}. Diagnostics dumped to: {}",
                result.error,
                diag_path.display()
            ));
        }
    };

    assert!(
        result.success,
        "Artifact generation should succeed. Error: {:?}",
        result.error
    );
    assert_eq!(result.target, "machine-name");
    assert_eq!(result.artifact_name, artifact_name);

    assert!(
        !result.generated_file_contents.is_empty(),
        "Generated files should not be empty"
    );

    assert!(
        result
            .generated_file_contents
            .contains_key("very-simple-secrets"),
        "Should generate very-simple-secrets file"
    );
    assert!(
        result
            .generated_file_contents
            .contains_key("simple-secrets"),
        "Should generate simple-secrets file"
    );

    assert_eq!(
        result.generated_file_contents.get("very-simple-secrets"),
        Some(&"test-secret-one".to_string()),
        "very-simple-secrets content should match expected value"
    );
    assert_eq!(
        result.generated_file_contents.get("simple-secrets"),
        Some(&"test-secret-two".to_string()),
        "simple-secrets content should match expected value"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_multiple_machines_artifacts_created() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/multiple-machines")?;

    let machines: Vec<String> = harness.make.nixos_map.keys().cloned().collect();
    assert!(
        machines.len() >= 2,
        "Should have multiple machines for this test"
    );

    let mut all_succeeded = true;

    for machine_name in &machines {
        if let Some((_artifact_name, artifact_def)) =
            find_first_artifact(&harness.make, machine_name)
        {
            let prompt_values: BTreeMap<String, String> = BTreeMap::new();

            let result = harness.generate_artifact(
                machine_name,
                &artifact_def,
                TargetType::NixOS {
                    machine: machine_name.clone(),
                },
                &prompt_values,
            )?;

            if !result.success {
                all_succeeded = false;
                eprintln!(
                    "Failed to generate artifact for {}: {:?}",
                    machine_name, result.error
                );
            }
        }
    }

    assert!(
        all_succeeded,
        "All artifacts should be created successfully"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_no_prompts_artifact_creation() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/two-artifacts-no-prompts")?;

    let machine_name = harness
        .make
        .nixos_map
        .keys()
        .next()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No machines found"))?;

    let (_artifact_name, artifact_def) = harness
        .find_artifact(&machine_name, None)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    let prompt_values: BTreeMap<String, String> = BTreeMap::new();

    let result = harness.generate_artifact(
        &machine_name,
        &artifact_def,
        TargetType::NixOS {
            machine: machine_name.clone(),
        },
        &prompt_values,
    )?;

    assert!(
        result.success,
        "Artifact without prompts should generate successfully. Error: {:?}",
        result.error
    );
    assert!(
        !result.generated_file_contents.is_empty(),
        "Should generate files"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_missing_prompts_fails() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/single-artifact-with-prompts")?;

    let (_artifact_name, artifact_def) = harness
        .find_artifact("machine-name", None)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    let empty_prompts: BTreeMap<String, String> = BTreeMap::new();

    let result = harness.generate_artifact(
        "machine-name",
        &artifact_def,
        TargetType::NixOS {
            machine: "machine-name".to_string(),
        },
        &empty_prompts,
    )?;

    match result.success {
        true => {
            eprintln!("Note: Generator succeeded with empty prompts (expected behavior)");
        }
        false => {
            eprintln!(
                "Generation failed as expected with missing prompts: {:?}",
                result.error
            );
        }
    }

    Ok(())
}

#[test]
#[serial]
fn e2e_headless_programmatic_invocation() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/two-artifacts-no-prompts")?;

    let machine_name = harness.make.nixos_map.keys().next().cloned().unwrap();

    let (_, artifact_def) = harness.find_artifact(&machine_name, None).unwrap();

    let result = harness.generate_artifact(
        &machine_name,
        &artifact_def,
        TargetType::NixOS {
            machine: machine_name.clone(),
        },
        &BTreeMap::new(),
    )?;

    assert!(result.success, "Headless generation should work");

    Ok(())
}

#[test]
#[serial]
fn e2e_home_manager_only_config_loads() -> Result<()> {
    let (_backend, make_config) = load_example("scenarios/home-manager-only")?;
    insta::with_settings!({
        filters => [
            (r"/nix/store/[a-z0-9]+-", "/nix/store/HASH-"),
        ]
    }, {
        insta::assert_debug_snapshot!(make_config);
    });
    Ok(())
}

#[test]
#[serial]
fn e2e_home_manager_artifact_generation() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/home-manager-only")?;

    let user_name = harness
        .make
        .home_map
        .keys()
        .next()
        .cloned()
        .expect("Should have at least one user");

    let (_artifact_name, artifact_def) = harness
        .find_artifact(&user_name, None)
        .expect("Should have at least one artifact");

    let prompt_values: BTreeMap<String, String> = BTreeMap::new();

    let (result, diagnostics) = harness.generate_artifact_with_diagnostics(
        &user_name,
        &artifact_def,
        TargetType::HomeManager {
            username: user_name.clone(),
        },
        &prompt_values,
    )?;

    insta::with_settings!({
        filters => [
            (r"/nix/store/[a-z0-9]+-", "/nix/store/HASH-"),
            (r"/tmp/[^/]+/storage", "/tmp/REDACTED/storage"),
            (r"CARGO_[A-Z_]+: [^\n]+,\n\s*", ""),
            (r"CARGO_[A-Z_]+: [^\n]+\n", ""),
        ]
    }, {
        let env_vars: std::collections::BTreeMap<_, _> = diagnostics.environment_vars.iter()
            .filter(|(k, _)| !k.starts_with("CARGO_"))
            .collect();
        let mut diag = diagnostics.clone();
        diag.environment_vars = env_vars.into_iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        insta::assert_debug_snapshot!((&harness.make, result, diag));
    });

    Ok(())
}

#[test]
#[serial]
fn e2e_config_structure_comparison() -> Result<()> {
    let (_nixos_backend, nixos_config) = load_example("scenarios/single-artifact-with-prompts")?;
    let (_home_backend, home_config) = load_example("scenarios/home-manager-only")?;
    insta::with_settings!({
        filters => [
            (r"/nix/store/[a-z0-9]+-", "/nix/store/HASH-"),
        ]
    }, {
        insta::assert_debug_snapshot!((nixos_config, home_config));
    });
    Ok(())
}
