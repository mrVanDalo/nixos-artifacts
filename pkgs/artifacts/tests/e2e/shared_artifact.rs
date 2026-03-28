//! Shared artifact tests for TEST-05.
//!
//! This module tests artifacts that are shared across multiple machines.
//! Shared artifacts have `shared = true` in their definition and are conceptually
//! the same across all machines that reference them.
//!
//! In headless mode, shared artifacts are generated using the normal serialize
//! path (stored in machines/{machine}/) because headless generates per-machine.
//! The "shared" aspect is that multiple machines reference the same artifact
//! definition and would receive the same content.
//!
//! Test Requirements:
//! - TEST-05: Test covers single and shared artifacts
//! - TEST-06: Tests run in CI with meaningful failures
//!
//! How to run:
//! - All e2e tests: cargo test --test tests e2e
//! - Shared artifact tests: cargo test --test tests e2e_shared_artifact
//! - Specific test: cargo test --test tests e2e_shared_artifact_generation
//!
//! Prerequisites:
//! - Nix installation with flake support
//! - Shared artifact scenarios in examples/scenarios/shared-artifacts

use crate::common::TestHarness;
use anyhow::{Context, Result};
use artifacts::app::model::TargetType;
use artifacts::config::make::ArtifactDef;
use serial_test::serial;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn get_machine_artifact_path(
    storage_dir: &Path,
    machine_name: &str,
    artifact_name: &str,
) -> PathBuf {
    storage_dir
        .join("machines")
        .join(machine_name)
        .join(artifact_name)
}

fn find_shared_artifact(
    harness: &TestHarness,
    artifact_name: &str,
) -> Option<(String, ArtifactDef)> {
    for (machine_name, artifacts) in &harness.make.nixos_map {
        if let Some(def) = artifacts.get(artifact_name)
            && def.shared
        {
            return Some((machine_name.clone(), def.clone()));
        }
    }
    None
}

fn find_machine_artifact(
    harness: &TestHarness,
    machine_name: &str,
    artifact_name: &str,
) -> Option<ArtifactDef> {
    harness
        .make
        .nixos_map
        .get(machine_name)
        .and_then(|artifacts| artifacts.get(artifact_name))
        .filter(|def| !def.shared)
        .cloned()
}

fn verify_artifact_exists(
    storage_dir: &Path,
    machine_name: &str,
    artifact_name: &str,
    file_name: &str,
) -> Result<PathBuf> {
    let artifact_path = get_machine_artifact_path(storage_dir, machine_name, artifact_name);
    let file_path = artifact_path.join(file_name);

    if !file_path.exists() {
        let parent_contents: Vec<String> = if artifact_path.exists() {
            fs::read_dir(&artifact_path)
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .collect()
                })
                .unwrap_or_default()
        } else {
            let machine_path = storage_dir.join("machines").join(machine_name);
            fs::read_dir(&machine_path)
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .collect()
                })
                .unwrap_or_else(|| vec!["(machine directory not found)".to_string()])
        };

        return Err(anyhow::anyhow!(
            "Artifact file '{}' not found at expected path: {}. Parent contents: {:?}",
            file_name,
            file_path.display(),
            parent_contents
        ));
    }

    Ok(file_path)
}

#[test]
#[serial]
fn e2e_shared_artifact_generation() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/shared-artifacts")?;

    let (machine_one, shared_def) = find_shared_artifact(&harness, "shared-secret")
        .ok_or_else(|| anyhow::anyhow!("Shared artifact 'shared-secret' not found"))?;

    assert!(shared_def.shared, "Artifact should be marked as shared");

    let prompt_values: BTreeMap<String, String> = BTreeMap::new();

    let result = harness.generate_artifact(
        &machine_one,
        &shared_def,
        TargetType::NixOS {
            machine: machine_one.clone(),
        },
        &prompt_values,
    )?;

    assert!(
        result.success,
        "Shared artifact generation should succeed. Error: {:?}",
        result.error
    );
    assert_eq!(result.artifact_name, "shared-secret");

    assert!(
        !result.generated_file_contents.is_empty(),
        "Generated files should not be empty"
    );
    assert!(
        result.generated_file_contents.contains_key("shared-key"),
        "Should generate shared-key file"
    );
    assert_eq!(
        result
            .generated_file_contents
            .get("shared-key")
            .map(|s| s.trim()),
        Some("shared-value"),
        "shared-key content should match expected value"
    );

    let storage_path = harness.temp_dir.path().join("storage");
    let artifact_file = verify_artifact_exists(
        &storage_path,
        &machine_one,
        "shared-secret",
        "shared-key",
    )
    .with_context(|| {
        format!(
            "Shared artifact 'shared-secret' should exist in backend storage for machine '{}'",
            machine_one
        )
    })?;

    let content = fs::read_to_string(&artifact_file)?;
    assert_eq!(
        content.trim(),
        "shared-value",
        "Shared artifact should have correct content"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_shared_artifact_multi_machine() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/shared-artifacts")?;

    let machines: Vec<String> = harness.make.nixos_map.keys().cloned().collect();
    assert!(
        machines.len() >= 2,
        "Should have at least 2 machines for shared artifact test"
    );

    let machine_one = machines
        .iter()
        .find(|m| *m == "machine-one")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("machine-one not found"))?;

    let machine_two = machines
        .iter()
        .find(|m| *m == "machine-two")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("machine-two not found"))?;

    let (_, shared_def) = find_shared_artifact(&harness, "shared-secret")
        .ok_or_else(|| anyhow::anyhow!("Shared artifact not found"))?;

    let prompt_values: BTreeMap<String, String> = BTreeMap::new();

    let result1 = harness.generate_artifact(
        &machine_one,
        &shared_def,
        TargetType::NixOS {
            machine: machine_one.clone(),
        },
        &prompt_values,
    )?;

    assert!(
        result1.success,
        "Shared artifact generation for machine-one should succeed"
    );

    let result2 = harness.generate_artifact(
        &machine_two,
        &shared_def,
        TargetType::NixOS {
            machine: machine_two.clone(),
        },
        &prompt_values,
    )?;

    assert!(
        result2.success,
        "Shared artifact generation for machine-two should succeed"
    );

    let storage_path = harness.temp_dir.path().join("storage");
    let file1 = verify_artifact_exists(&storage_path, &machine_one, "shared-secret", "shared-key")?;
    let file2 = verify_artifact_exists(&storage_path, &machine_two, "shared-secret", "shared-key")?;

    let content1 = fs::read_to_string(&file1)?;
    let content2 = fs::read_to_string(&file2)?;

    assert_eq!(
        content1.trim(),
        "shared-value",
        "Machine-one shared artifact should have correct content"
    );
    assert_eq!(
        content2.trim(),
        "shared-value",
        "Machine-two shared artifact should have correct content"
    );

    assert_eq!(
        content1, content2,
        "Both machines should have identical shared artifact content"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_shared_artifact_single_instance() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/shared-artifacts")?;

    let machine_one = harness
        .make
        .nixos_map
        .keys()
        .find(|k| *k == "machine-one")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("machine-one not found"))?;

    let (_, shared_def) = find_shared_artifact(&harness, "shared-secret")
        .ok_or_else(|| anyhow::anyhow!("Shared artifact not found"))?;

    let prompt_values: BTreeMap<String, String> = BTreeMap::new();

    let result = harness.generate_artifact(
        &machine_one,
        &shared_def,
        TargetType::NixOS {
            machine: machine_one.clone(),
        },
        &prompt_values,
    )?;

    assert!(result.success, "Generation should succeed");

    let storage_path = harness.temp_dir.path().join("storage");
    let artifact_file =
        verify_artifact_exists(&storage_path, &machine_one, "shared-secret", "shared-key")?;

    let content = fs::read_to_string(&artifact_file)?;
    assert_eq!(
        content.trim(),
        "shared-value",
        "Shared artifact should have correct content"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_shared_vs_machine_artifacts() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/shared-artifacts")?;

    let machine_one = harness
        .make
        .nixos_map
        .keys()
        .find(|k| *k == "machine-one")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("machine-one not found"))?;

    let machine_two = harness
        .make
        .nixos_map
        .keys()
        .find(|k| *k == "machine-two")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("machine-two not found"))?;

    let shared_def = find_shared_artifact(&harness, "shared-secret")
        .map(|(_, def)| def)
        .ok_or_else(|| anyhow::anyhow!("shared-secret not found"))?;

    assert!(
        shared_def.shared,
        "shared-secret should be marked as shared"
    );

    let local_one = find_machine_artifact(&harness, &machine_one, "local-secret")
        .ok_or_else(|| anyhow::anyhow!("local-secret not found for machine-one"))?;

    let local_two = find_machine_artifact(&harness, &machine_two, "local-secret")
        .ok_or_else(|| anyhow::anyhow!("local-secret not found for machine-two"))?;

    assert!(
        !local_one.shared,
        "local-secret should not be marked as shared"
    );
    assert!(
        !local_two.shared,
        "local-secret should not be marked as shared"
    );

    let prompt_values: BTreeMap<String, String> = BTreeMap::new();

    let shared_result = harness.generate_artifact(
        &machine_one,
        &shared_def,
        TargetType::NixOS {
            machine: machine_one.clone(),
        },
        &prompt_values,
    )?;
    assert!(
        shared_result.success,
        "Shared artifact generation should succeed"
    );

    let local_one_result = harness.generate_artifact(
        &machine_one,
        &local_one,
        TargetType::NixOS {
            machine: machine_one.clone(),
        },
        &prompt_values,
    )?;
    assert!(
        local_one_result.success,
        "Machine-one specific artifact generation should succeed"
    );

    let local_two_result = harness.generate_artifact(
        &machine_two,
        &local_two,
        TargetType::NixOS {
            machine: machine_two.clone(),
        },
        &prompt_values,
    )?;
    assert!(
        local_two_result.success,
        "Machine-two specific artifact generation should succeed"
    );

    let storage_path = harness.temp_dir.path().join("storage");
    let shared_file =
        verify_artifact_exists(&storage_path, &machine_one, "shared-secret", "shared-key")?;

    let local_one_file =
        verify_artifact_exists(&storage_path, &machine_one, "local-secret", "local-key")?;
    let local_two_file =
        verify_artifact_exists(&storage_path, &machine_two, "local-secret", "local-key")?;

    let shared_content = fs::read_to_string(&shared_file)?;
    let local_one_content = fs::read_to_string(&local_one_file)?;
    let local_two_content = fs::read_to_string(&local_two_file)?;

    assert_eq!(
        shared_content.trim(),
        "shared-value",
        "Shared content should match expected"
    );
    assert_eq!(
        local_one_content.trim(),
        "local-one",
        "Machine-one local content should match expected"
    );
    assert_eq!(
        local_two_content.trim(),
        "local-two",
        "Machine-two local content should match expected"
    );

    assert_ne!(
        local_one_content.trim(),
        local_two_content.trim(),
        "Machine-specific artifacts should have different content"
    );

    assert_ne!(
        shared_content.trim(),
        local_one_content.trim(),
        "Shared and machine-specific content should be different"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_shared_artifact_consistency() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/shared-artifacts")?;

    let machines: Vec<String> = harness.make.nixos_map.keys().cloned().collect();
    let machine_one = machines
        .iter()
        .find(|m| *m == "machine-one")
        .cloned()
        .unwrap_or_else(|| machines[0].clone());

    let machine_two = machines
        .iter()
        .find(|m| *m == "machine-two")
        .cloned()
        .unwrap_or_else(|| machines[1].clone());

    let (_, shared_def) = find_shared_artifact(&harness, "shared-secret")
        .ok_or_else(|| anyhow::anyhow!("Shared artifact not found"))?;

    let prompt_values: BTreeMap<String, String> = BTreeMap::new();

    let result1 = harness.generate_artifact(
        &machine_one,
        &shared_def,
        TargetType::NixOS {
            machine: machine_one.clone(),
        },
        &prompt_values,
    )?;
    assert!(result1.success, "Generation for machine-one should succeed");

    let result2 = harness.generate_artifact(
        &machine_two,
        &shared_def,
        TargetType::NixOS {
            machine: machine_two.clone(),
        },
        &prompt_values,
    )?;
    assert!(result2.success, "Generation for machine-two should succeed");

    let storage_path = harness.temp_dir.path().join("storage");
    let file1 = verify_artifact_exists(&storage_path, &machine_one, "shared-secret", "shared-key")?;
    let file2 = verify_artifact_exists(&storage_path, &machine_two, "shared-secret", "shared-key")?;

    let content1 = fs::read_to_string(&file1)?;
    let content2 = fs::read_to_string(&file2)?;

    assert_eq!(
        content1, content2,
        "Shared artifacts for different machines should have identical content"
    );

    assert_eq!(
        content1.trim(),
        "shared-value",
        "Shared artifact should have expected content"
    );

    for machine in &machines {
        if let Some(artifacts) = harness.make.nixos_map.get(machine) {
            let def = artifacts
                .get("shared-secret")
                .expect("All machines should have shared-secret");
            assert!(
                def.shared,
                "shared-secret on {} should be marked as shared",
                machine
            );
        }
    }

    Ok(())
}
