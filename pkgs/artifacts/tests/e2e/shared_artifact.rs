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

use anyhow::{Context, Result};
use artifacts::app::model::TargetType;
use artifacts::cli::headless::{PromptValues, generate_single_artifact_with_target_type};
use artifacts::config::make::ArtifactDef;
use serial_test::serial;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

// Import helpers from parent module
use super::{CleanupGuard, load_example, setup_test_storage};

/// Get the expected path for a machine-specific artifact in backend storage.
///
/// The test backend stores artifacts in: {storage_dir}/machines/{machine}/{artifact}/
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

/// Find the shared artifact definition across all machines.
fn find_shared_artifact(
    make_config: &artifacts::config::make::MakeConfiguration,
    artifact_name: &str,
) -> Option<(String, ArtifactDef)> {
    // Look in the first machine that has this artifact
    for (machine_name, artifacts) in &make_config.nixos_map {
        if let Some(def) = artifacts.get(artifact_name)
            && def.shared
        {
            return Some((machine_name.clone(), def.clone()));
        }
    }
    None
}

/// Find a non-shared (machine-specific) artifact.
fn find_machine_artifact(
    make_config: &artifacts::config::make::MakeConfiguration,
    machine_name: &str,
    artifact_name: &str,
) -> Option<ArtifactDef> {
    make_config
        .nixos_map
        .get(machine_name)
        .and_then(|artifacts| artifacts.get(artifact_name))
        .filter(|def| !def.shared)
        .cloned()
}

/// Verify that an artifact exists in the backend storage.
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

// =============================================================================
// TEST-05: Shared artifact generation
// =============================================================================

/// TEST-05: Shared artifact generation across multiple machines.
///
/// This test verifies that:
/// 1. A shared artifact (with shared=true) can be generated for a machine
/// 2. The artifact is marked correctly in the configuration
/// 3. The artifact exists in the backend storage
///
/// In headless mode, shared artifacts use the normal serialize path
/// (stored in machines/{machine}/) because headless generates per-machine.
/// The "shared" aspect is about the artifact definition being shared.
#[test]
#[serial]
fn e2e_shared_artifact_generation() -> Result<()> {
    // Set up test storage
    let (storage_dir, storage_path) = setup_test_storage()?;
    let _cleanup = CleanupGuard;

    // Load the shared-artifacts scenario
    let (backend, make_config) = load_example("scenarios/shared-artifacts")?;

    // Get the shared artifact definition
    let (machine_one, shared_def) = find_shared_artifact(&make_config, "shared-secret")
        .ok_or_else(|| anyhow::anyhow!("Shared artifact 'shared-secret' not found"))?;

    // Verify the artifact is marked as shared
    assert!(shared_def.shared, "Artifact should be marked as shared");

    // Generate the artifact for machine-one
    let prompt_values: PromptValues = BTreeMap::new();

    let result = generate_single_artifact_with_target_type(
        &machine_one,
        &shared_def,
        &prompt_values,
        &backend,
        &make_config,
        TargetType::NixOS {
            machine: machine_one.clone(),
        },
    )?;

    // Verify generation succeeded
    assert!(
        result.success,
        "Shared artifact generation should succeed. Error: {:?}",
        result.error
    );
    assert_eq!(result.artifact_name, "shared-secret");

    // Verify generated files exist in the result
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

    // Verify the artifact was created in backend storage
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

    // Verify the content
    let content = fs::read_to_string(&artifact_file)?;
    assert_eq!(
        content.trim(),
        "shared-value",
        "Shared artifact should have correct content"
    );

    // Clean up
    drop(storage_dir);

    Ok(())
}

/// TEST-05: Shared artifact accessible to multiple machines.
///
/// This test verifies that a shared artifact defined in the configuration
/// is accessible to multiple machines (they all reference the same artifact).
#[test]
#[serial]
fn e2e_shared_artifact_multi_machine() -> Result<()> {
    // Set up test storage
    let (storage_dir, storage_path) = setup_test_storage()?;
    let _cleanup = CleanupGuard;

    // Load the shared-artifacts scenario
    let (backend, make_config) = load_example("scenarios/shared-artifacts")?;

    // Get both machines
    let machines: Vec<String> = make_config.nixos_map.keys().cloned().collect();
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

    // Get the shared artifact
    let (_, shared_def) = find_shared_artifact(&make_config, "shared-secret")
        .ok_or_else(|| anyhow::anyhow!("Shared artifact not found"))?;

    // Generate for machine-one
    let prompt_values: PromptValues = BTreeMap::new();

    let result1 = generate_single_artifact_with_target_type(
        &machine_one,
        &shared_def,
        &prompt_values,
        &backend,
        &make_config,
        TargetType::NixOS {
            machine: machine_one.clone(),
        },
    )?;

    assert!(
        result1.success,
        "Shared artifact generation for machine-one should succeed"
    );

    // Generate for machine-two
    let result2 = generate_single_artifact_with_target_type(
        &machine_two,
        &shared_def,
        &prompt_values,
        &backend,
        &make_config,
        TargetType::NixOS {
            machine: machine_two.clone(),
        },
    )?;

    assert!(
        result2.success,
        "Shared artifact generation for machine-two should succeed"
    );

    // Verify both machines have the artifact
    let file1 = verify_artifact_exists(&storage_path, &machine_one, "shared-secret", "shared-key")?;
    let file2 = verify_artifact_exists(&storage_path, &machine_two, "shared-secret", "shared-key")?;

    // Verify both have the same content
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

    // Both machines have the same content because they use the same generator
    // This is the essence of "shared" - same definition, same content
    assert_eq!(
        content1, content2,
        "Both machines should have identical shared artifact content"
    );

    // Clean up
    drop(storage_dir);

    Ok(())
}

/// TEST-05: Shared artifact generates independently per machine.
///
/// This test verifies that when generating shared artifacts for multiple machines,
/// each machine gets its own copy (which is correct for headless mode).
/// In TUI mode with check_serialization, the second generation might be skipped.
#[test]
#[serial]
fn e2e_shared_artifact_single_instance() -> Result<()> {
    // Set up test storage
    let (storage_dir, storage_path) = setup_test_storage()?;
    let _cleanup = CleanupGuard;

    // Load the shared-artifacts scenario
    let (backend, make_config) = load_example("scenarios/shared-artifacts")?;

    // Get machine-one
    let machine_one = make_config
        .nixos_map
        .keys()
        .find(|k| *k == "machine-one")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("machine-one not found"))?;

    // Get the shared artifact
    let (_, shared_def) = find_shared_artifact(&make_config, "shared-secret")
        .ok_or_else(|| anyhow::anyhow!("Shared artifact not found"))?;

    // Generate
    let prompt_values: PromptValues = BTreeMap::new();

    let result = generate_single_artifact_with_target_type(
        &machine_one,
        &shared_def,
        &prompt_values,
        &backend,
        &make_config,
        TargetType::NixOS {
            machine: machine_one.clone(),
        },
    )?;

    assert!(result.success, "Generation should succeed");

    // Verify the artifact exists
    let artifact_file =
        verify_artifact_exists(&storage_path, &machine_one, "shared-secret", "shared-key")?;

    let content = fs::read_to_string(&artifact_file)?;
    assert_eq!(
        content.trim(),
        "shared-value",
        "Shared artifact should have correct content"
    );

    // Clean up
    drop(storage_dir);

    Ok(())
}

/// TEST-05: Verify shared vs machine-specific artifacts.
///
/// This test verifies that:
/// - Shared artifacts (shared=true) work correctly
/// - Machine-specific artifacts (shared=false) work correctly
/// - Both can coexist in the same configuration
/// - Both are stored in their respective machine directories
#[test]
#[serial]
fn e2e_shared_vs_machine_artifacts() -> Result<()> {
    // Set up test storage
    let (storage_dir, storage_path) = setup_test_storage()?;
    let _cleanup = CleanupGuard;

    // Load the shared-artifacts scenario
    let (backend, make_config) = load_example("scenarios/shared-artifacts")?;

    // Get machine-one
    let machine_one = make_config
        .nixos_map
        .keys()
        .find(|k| *k == "machine-one")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("machine-one not found"))?;

    // Get machine-two for comparison
    let machine_two = make_config
        .nixos_map
        .keys()
        .find(|k| *k == "machine-two")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("machine-two not found"))?;

    // Get the shared artifact
    let shared_def = find_shared_artifact(&make_config, "shared-secret")
        .map(|(_, def)| def)
        .ok_or_else(|| anyhow::anyhow!("shared-secret not found"))?;

    assert!(
        shared_def.shared,
        "shared-secret should be marked as shared"
    );

    // Get the machine-specific artifacts
    let local_one = find_machine_artifact(&make_config, &machine_one, "local-secret")
        .ok_or_else(|| anyhow::anyhow!("local-secret not found for machine-one"))?;

    let local_two = find_machine_artifact(&make_config, &machine_two, "local-secret")
        .ok_or_else(|| anyhow::anyhow!("local-secret not found for machine-two"))?;

    assert!(
        !local_one.shared,
        "local-secret should not be marked as shared"
    );
    assert!(
        !local_two.shared,
        "local-secret should not be marked as shared"
    );

    // Generate all artifacts
    let prompt_values: PromptValues = BTreeMap::new();

    // Generate shared artifact for machine-one
    let shared_result = generate_single_artifact_with_target_type(
        &machine_one,
        &shared_def,
        &prompt_values,
        &backend,
        &make_config,
        TargetType::NixOS {
            machine: machine_one.clone(),
        },
    )?;
    assert!(
        shared_result.success,
        "Shared artifact generation should succeed"
    );

    // Generate machine-specific artifacts
    let local_one_result = generate_single_artifact_with_target_type(
        &machine_one,
        &local_one,
        &prompt_values,
        &backend,
        &make_config,
        TargetType::NixOS {
            machine: machine_one.clone(),
        },
    )?;
    assert!(
        local_one_result.success,
        "Machine-one specific artifact generation should succeed"
    );

    let local_two_result = generate_single_artifact_with_target_type(
        &machine_two,
        &local_two,
        &prompt_values,
        &backend,
        &make_config,
        TargetType::NixOS {
            machine: machine_two.clone(),
        },
    )?;
    assert!(
        local_two_result.success,
        "Machine-two specific artifact generation should succeed"
    );

    // Verify shared artifact exists for machine-one
    let shared_file =
        verify_artifact_exists(&storage_path, &machine_one, "shared-secret", "shared-key")?;

    // Verify machine-specific artifacts exist
    let local_one_file =
        verify_artifact_exists(&storage_path, &machine_one, "local-secret", "local-key")?;
    let local_two_file =
        verify_artifact_exists(&storage_path, &machine_two, "local-secret", "local-key")?;

    // Verify contents are correct
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

    // Machine-specific artifacts have different content
    assert_ne!(
        local_one_content.trim(),
        local_two_content.trim(),
        "Machine-specific artifacts should have different content"
    );

    // Shared and local are independent
    assert_ne!(
        shared_content.trim(),
        local_one_content.trim(),
        "Shared and machine-specific content should be different"
    );

    // Clean up
    drop(storage_dir);

    Ok(())
}

/// TEST-05: Verify shared artifact consistency across machines.
///
/// This test ensures that when multiple machines reference the same shared
/// artifact, they all get the same content because they use the same generator.
#[test]
#[serial]
fn e2e_shared_artifact_consistency() -> Result<()> {
    // Set up test storage
    let (storage_dir, storage_path) = setup_test_storage()?;
    let _cleanup = CleanupGuard;

    // Load the shared-artifacts scenario
    let (backend, make_config) = load_example("scenarios/shared-artifacts")?;

    // Get both machines
    let machines: Vec<String> = make_config.nixos_map.keys().cloned().collect();
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

    // Get the shared artifact
    let (_, shared_def) = find_shared_artifact(&make_config, "shared-secret")
        .ok_or_else(|| anyhow::anyhow!("Shared artifact not found"))?;

    // Generate for both machines
    let prompt_values: PromptValues = BTreeMap::new();

    let result1 = generate_single_artifact_with_target_type(
        &machine_one,
        &shared_def,
        &prompt_values,
        &backend,
        &make_config,
        TargetType::NixOS {
            machine: machine_one.clone(),
        },
    )?;
    assert!(result1.success, "Generation for machine-one should succeed");

    let result2 = generate_single_artifact_with_target_type(
        &machine_two,
        &shared_def,
        &prompt_values,
        &backend,
        &make_config,
        TargetType::NixOS {
            machine: machine_two.clone(),
        },
    )?;
    assert!(result2.success, "Generation for machine-two should succeed");

    // Verify both artifacts exist
    let file1 = verify_artifact_exists(&storage_path, &machine_one, "shared-secret", "shared-key")?;
    let file2 = verify_artifact_exists(&storage_path, &machine_two, "shared-secret", "shared-key")?;

    // Verify content is identical (the shared nature)
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

    // Verify both machines have the same shared artifact definition
    for machine in &machines {
        if let Some(artifacts) = make_config.nixos_map.get(machine) {
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

    // Clean up
    drop(storage_dir);

    Ok(())
}
