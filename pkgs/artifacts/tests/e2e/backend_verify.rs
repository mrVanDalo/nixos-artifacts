//! Backend storage verification tests for artifacts.
//!
//! This module focuses specifically on verifying that artifacts are actually
//! stored in the backend with correct content and format.
//!
//! Test Requirements:
//! - TEST-03: Verify artifact exists at expected backend location
//! - TEST-04: Verify artifact content matches expected format
//!
//! How to run:
//! - All e2e tests: cargo test --test tests e2e
//! - Backend storage tests: cargo test --test tests e2e_backend_storage
//! - Specific test: cargo test --test tests e2e_backend_storage_single_artifact
//!
//! Prerequisites:
//! - Nix installation with flake support
//! - Scenarios in examples/scenarios/ directory

use crate::common::TestHarness;
use anyhow::{Context, Result};
use artifacts::app::model::TargetType;
use serial_test::serial;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn get_artifact_storage_path(
    storage_dir: &std::path::Path,
    machine_name: &str,
    artifact_name: &str,
) -> PathBuf {
    storage_dir
        .join("machines")
        .join(machine_name)
        .join(artifact_name)
}

fn verify_artifact_in_storage(
    storage_dir: &std::path::Path,
    machine_name: &str,
    artifact_name: &str,
) -> Result<PathBuf> {
    let artifact_path = get_artifact_storage_path(storage_dir, machine_name, artifact_name);

    if !artifact_path.exists() {
        let parent = artifact_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let contents: Vec<String> = if parent.is_empty() {
            vec![]
        } else {
            fs::read_dir(&parent)
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .collect()
                })
                .unwrap_or_default()
        };

        return Err(anyhow::anyhow!(
            "Artifact '{}' not found at expected path: {}. Parent directory contents: {:?}",
            artifact_name,
            artifact_path.display(),
            contents
        ));
    }

    Ok(artifact_path)
}

fn verify_file_in_artifact(
    artifact_path: &std::path::Path,
    filename: &str,
    expected_content: &str,
) -> Result<()> {
    let file_path = artifact_path.join(filename);

    if !file_path.exists() {
        let contents: Vec<String> = fs::read_dir(artifact_path)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();

        return Err(anyhow::anyhow!(
            "File '{}' not found in artifact at {}. Contents: {:?}",
            filename,
            artifact_path.display(),
            contents
        ));
    }

    let actual_content = fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    if actual_content != expected_content {
        return Err(anyhow::anyhow!(
            "File '{}' content mismatch. Expected: {:?}, Actual: {:?}",
            filename,
            expected_content,
            actual_content
        ));
    }

    Ok(())
}

#[test]
#[serial]
fn e2e_backend_storage_single_artifact() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/single-artifact-with-prompts")?;

    let (artifact_name, artifact_def) = harness
        .find_artifact("machine-name", None)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found for machine-name"))?;

    let prompt_values: BTreeMap<String, String> = BTreeMap::from([
        ("secret1".to_string(), "test-secret-one".to_string()),
        ("secret2".to_string(), "test-secret-two".to_string()),
    ]);

    let result = harness.generate_artifact(
        "machine-name",
        &artifact_def,
        TargetType::NixOS {
            machine: "machine-name".to_string(),
        },
        &prompt_values,
    )?;

    assert!(result.success, "Artifact generation should succeed");

    let storage_path = harness.temp_dir.path().join("storage");
    let artifact_path = verify_artifact_in_storage(&storage_path, "machine-name", &artifact_name)
        .with_context(|| {
        format!(
            "Artifact '{}' should exist in backend storage for machine 'machine-name'",
            artifact_name
        )
    })?;

    let entries: Vec<_> = fs::read_dir(&artifact_path)?
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        !entries.is_empty(),
        "Artifact directory should not be empty"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_backend_storage_content_format() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/single-artifact-with-prompts")?;

    let (artifact_name, artifact_def) = harness
        .find_artifact("machine-name", None)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found for machine-name"))?;

    let expected_content1 = "known-secret-value-one";
    let expected_content2 = "known-secret-value-two";

    let prompt_values: BTreeMap<String, String> = BTreeMap::from([
        ("secret1".to_string(), expected_content1.to_string()),
        ("secret2".to_string(), expected_content2.to_string()),
    ]);

    let result = harness.generate_artifact(
        "machine-name",
        &artifact_def,
        TargetType::NixOS {
            machine: "machine-name".to_string(),
        },
        &prompt_values,
    )?;

    assert!(result.success, "Artifact generation should succeed");

    let storage_path = harness.temp_dir.path().join("storage");
    let artifact_path = verify_artifact_in_storage(&storage_path, "machine-name", &artifact_name)?;

    verify_file_in_artifact(&artifact_path, "very-simple-secrets", expected_content1)
        .with_context(|| "First file should have correct content")?;

    verify_file_in_artifact(&artifact_path, "simple-secrets", expected_content2)
        .with_context(|| "Second file should have correct content")?;

    Ok(())
}

#[test]
#[serial]
fn e2e_backend_storage_multiple_files() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/single-artifact-with-prompts")?;

    let (artifact_name, artifact_def) = harness
        .find_artifact("machine-name", None)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    let prompt_values: BTreeMap<String, String> = BTreeMap::from([
        ("secret1".to_string(), "file-one-content".to_string()),
        ("secret2".to_string(), "file-two-content".to_string()),
    ]);

    let result = harness.generate_artifact(
        "machine-name",
        &artifact_def,
        TargetType::NixOS {
            machine: "machine-name".to_string(),
        },
        &prompt_values,
    )?;

    assert!(result.success, "Artifact generation should succeed");

    assert!(
        result.generated_file_contents.len() >= 2,
        "Should generate at least 2 files, got {}",
        result.generated_file_contents.len()
    );

    assert_eq!(
        result.generated_file_contents.get("very-simple-secrets"),
        Some(&"file-one-content".to_string()),
        "First file should have correct content"
    );
    assert_eq!(
        result.generated_file_contents.get("simple-secrets"),
        Some(&"file-two-content".to_string()),
        "Second file should have correct content"
    );

    let storage_path = harness.temp_dir.path().join("storage");
    let artifact_path = verify_artifact_in_storage(&storage_path, "machine-name", &artifact_name)
        .with_context(|| "Artifact should be stored in backend")?;

    assert!(
        artifact_path.join("very-simple-secrets").exists(),
        "First file should exist in storage"
    );
    assert!(
        artifact_path.join("simple-secrets").exists(),
        "Second file should exist in storage"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_backend_storage_persists() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/single-artifact-with-prompts")?;

    let (artifact_name, artifact_def) = harness
        .find_artifact("machine-name", None)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    let prompt_values: BTreeMap<String, String> = BTreeMap::from([
        ("secret1".to_string(), "persistent-value".to_string()),
        (
            "secret2".to_string(),
            "another-persistent-value".to_string(),
        ),
    ]);

    let result = harness.generate_artifact(
        "machine-name",
        &artifact_def,
        TargetType::NixOS {
            machine: "machine-name".to_string(),
        },
        &prompt_values,
    )?;

    assert!(result.success, "Generation should succeed");

    let storage_path = harness.temp_dir.path().join("storage");
    let artifact_path = verify_artifact_in_storage(&storage_path, "machine-name", &artifact_name)?;

    assert!(
        artifact_path.exists(),
        "Artifact should persist in storage after generation"
    );

    let content = fs::read_to_string(artifact_path.join("very-simple-secrets"))?;
    assert!(
        !content.is_empty(),
        "Persisted artifact should have content"
    );
    assert_eq!(
        content, "persistent-value",
        "Content should match what was generated"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_backend_storage_no_prompts() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/two-artifacts-no-prompts")?;

    let machine_name = harness
        .make
        .nixos_map
        .keys()
        .next()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No machines found"))?;

    let (artifact_name, artifact_def) = harness
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

    assert!(result.success, "Artifact without prompts should succeed");
    assert!(
        !result.generated_file_contents.is_empty(),
        "Should generate files"
    );

    let storage_path = harness.temp_dir.path().join("storage");
    verify_artifact_in_storage(&storage_path, &machine_name, &artifact_name)
        .with_context(|| "Artifact without prompts should be stored")?;

    Ok(())
}
