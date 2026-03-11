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

use anyhow::{Context, Result};
use artifacts::cli::headless::{PromptValues, generate_single_artifact};
use serial_test::serial;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// Import helpers from parent module
use super::{create_test_storage_dir, find_first_artifact, load_example};

/// Get the test backend output directory from environment or use a default.
///
/// The test backend uses ARTIFACTS_TEST_OUTPUT_DIR environment variable
/// to determine where to store serialized artifacts.
#[allow(dead_code)]
fn get_test_backend_output_dir() -> PathBuf {
    std::env::var("ARTIFACTS_TEST_OUTPUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // Use the actual test backend directory
            PathBuf::from("/tmp/artifacts-test")
        })
}

/// Get the expected path for an artifact in the test backend.
///
/// The test backend stores artifacts in: `{storage_dir}/machines/{machine}/{artifact}/`
/// where each artifact is a directory containing the serialized files.
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

/// Set up test storage directory and configure environment for test backend.
///
/// Returns a TempDir that should be kept alive for the duration of the test
/// to ensure the storage directory exists.
fn setup_test_storage(_test_name: &str) -> Result<(TempDir, PathBuf)> {
    let temp_dir = create_test_storage_dir(_test_name)?;
    let storage_path = temp_dir.path().join("storage");

    // Set the environment variable that the test backend uses
    // SAFETY: We're in a single-threaded test environment with #[serial]
    unsafe {
        std::env::set_var("ARTIFACTS_TEST_OUTPUT_DIR", &storage_path);
    }

    Ok((temp_dir, storage_path))
}

/// Clean up environment after test.
fn cleanup_test_storage() {
    // Note: The actual temp directory is cleaned up when TempDir is dropped
    // We just clear the environment variable here
    // SAFETY: We're in a single-threaded test environment with #[serial]
    unsafe {
        std::env::remove_var("ARTIFACTS_TEST_OUTPUT_DIR");
    }
}

/// Verify that an artifact exists in the backend storage.
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

/// Verify that a file exists in the artifact storage with expected content.
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

// =============================================================================
// TEST-03: Verify artifact exists at backend location
// =============================================================================

/// TEST-03: Verify single artifact exists at expected backend storage location.
///
/// This test generates an artifact and verifies it actually exists in the
/// backend storage directory after successful generation.
#[test]
#[serial]
fn e2e_backend_storage_single_artifact() -> Result<()> {
    // TEST-03: Setup test storage and configure backend
    let (storage_dir, storage_path) = setup_test_storage("single_artifact")?;
    let _cleanup = CleanupGuard;

    // Load the single-artifact-with-prompts scenario
    let (backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    // Find the first artifact for the machine
    let (artifact_name, artifact_def) = find_first_artifact(&make_config, "machine-name")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found for machine-name"))?;

    // Prepare prompt values
    let prompt_values: PromptValues = BTreeMap::from([
        ("secret1".to_string(), "test-secret-one".to_string()),
        ("secret2".to_string(), "test-secret-two".to_string()),
    ]);

    // Generate the artifact using headless API
    let result = generate_single_artifact(
        "machine-name",
        &artifact_def,
        &prompt_values,
        &backend,
        &make_config,
    )?;

    // Verify generation succeeded
    assert!(result.success, "Artifact generation should succeed");

    // TEST-03: Verify artifact exists at expected backend location
    // The test backend stores artifacts in: {storage}/machines/{machine}/{artifact}/
    let artifact_path = verify_artifact_in_storage(&storage_path, "machine-name", &artifact_name)
        .with_context(|| {
        format!(
            "Artifact '{}' should exist in backend storage for machine 'machine-name'",
            artifact_name
        )
    })?;

    // Verify the artifact directory is not empty
    let entries: Vec<_> = fs::read_dir(&artifact_path)?
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        !entries.is_empty(),
        "Artifact directory should not be empty"
    );

    // Clean up this specific test's storage
    drop(storage_dir);

    Ok(())
}

// =============================================================================
// TEST-04: Verify artifact content matches expected format
// =============================================================================

/// TEST-04: Verify artifact content matches expected format after serialization.
///
/// This test generates an artifact with known content and verifies that
/// the content stored in the backend matches exactly what the generator produced.
#[test]
#[serial]
fn e2e_backend_storage_content_format() -> Result<()> {
    // TEST-04: Setup test storage
    let (storage_dir, storage_path) = setup_test_storage("content_format")?;
    let _cleanup = CleanupGuard;

    // Load the single-artifact-with-prompts scenario
    let (backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    let (artifact_name, artifact_def) = find_first_artifact(&make_config, "machine-name")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found for machine-name"))?;

    // Prepare known prompt values
    let expected_content1 = "known-secret-value-one";
    let expected_content2 = "known-secret-value-two";

    let prompt_values: PromptValues = BTreeMap::from([
        ("secret1".to_string(), expected_content1.to_string()),
        ("secret2".to_string(), expected_content2.to_string()),
    ]);

    // Generate the artifact
    let result = generate_single_artifact(
        "machine-name",
        &artifact_def,
        &prompt_values,
        &backend,
        &make_config,
    )?;

    assert!(result.success, "Artifact generation should succeed");

    // TEST-04: Verify artifact exists in backend
    let artifact_path = verify_artifact_in_storage(&storage_path, "machine-name", &artifact_name)?;

    // Verify each generated file exists with correct content
    // The generator produces files named "very-simple-secrets" and "simple-secrets"
    verify_file_in_artifact(&artifact_path, "very-simple-secrets", expected_content1)
        .with_context(|| "First file should have correct content")?;

    verify_file_in_artifact(&artifact_path, "simple-secrets", expected_content2)
        .with_context(|| "Second file should have correct content")?;

    // Clean up
    drop(storage_dir);

    Ok(())
}

// =============================================================================
// Additional edge case tests
// =============================================================================

/// Verify artifact with multiple files stores all files correctly.
///
/// This test verifies that artifacts with multiple files are all stored
/// in the backend with their correct content.
#[test]
#[serial]
fn e2e_backend_storage_multiple_files() -> Result<()> {
    // Setup test storage
    let (storage_dir, storage_path) = setup_test_storage("multiple_files")?;
    let _cleanup = CleanupGuard;

    // Use single-artifact-with-prompts which generates multiple files
    let (backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    let (artifact_name, artifact_def) = find_first_artifact(&make_config, "machine-name")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    // Prepare prompt values for multiple files
    let prompt_values: PromptValues = BTreeMap::from([
        ("secret1".to_string(), "file-one-content".to_string()),
        ("secret2".to_string(), "file-two-content".to_string()),
    ]);

    // Generate the artifact
    let result = generate_single_artifact(
        "machine-name",
        &artifact_def,
        &prompt_values,
        &backend,
        &make_config,
    )?;

    assert!(result.success, "Artifact generation should succeed");

    // Verify multiple files were generated
    assert!(
        result.generated_file_contents.len() >= 2,
        "Should generate at least 2 files, got {}",
        result.generated_file_contents.len()
    );

    // Verify each file has the expected content in the result
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

    // Verify artifact exists in backend storage
    let artifact_path = verify_artifact_in_storage(&storage_path, "machine-name", &artifact_name)
        .with_context(|| "Artifact should be stored in backend")?;

    // Verify files exist in storage
    assert!(
        artifact_path.join("very-simple-secrets").exists(),
        "First file should exist in storage"
    );
    assert!(
        artifact_path.join("simple-secrets").exists(),
        "Second file should exist in storage"
    );

    // Clean up
    drop(storage_dir);

    Ok(())
}

/// Verify artifacts persist after generation completes.
///
/// This test verifies that artifacts remain in storage after the generation
/// process completes and temporary directories are cleaned up.
#[test]
#[serial]
fn e2e_backend_storage_persists() -> Result<()> {
    // Setup test storage
    let (storage_dir, storage_path) = setup_test_storage("persists")?;

    // Load and generate
    let (backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    let (artifact_name, artifact_def) = find_first_artifact(&make_config, "machine-name")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    let prompt_values: PromptValues = BTreeMap::from([
        ("secret1".to_string(), "persistent-value".to_string()),
        (
            "secret2".to_string(),
            "another-persistent-value".to_string(),
        ),
    ]);

    // Generate
    let result = generate_single_artifact(
        "machine-name",
        &artifact_def,
        &prompt_values,
        &backend,
        &make_config,
    )?;

    assert!(result.success, "Generation should succeed");

    // Verify artifact exists immediately after generation
    let artifact_path = verify_artifact_in_storage(&storage_path, "machine-name", &artifact_name)?;

    // Verify the artifact persists
    assert!(
        artifact_path.exists(),
        "Artifact should persist in storage after generation"
    );

    // Read the content to verify it persisted correctly
    let content = fs::read_to_string(artifact_path.join("very-simple-secrets"))?;
    assert!(
        !content.is_empty(),
        "Persisted artifact should have content"
    );
    assert_eq!(
        content, "persistent-value",
        "Content should match what was generated"
    );

    // Clean up
    drop(storage_dir);

    Ok(())
}

/// Verify artifact generation with no prompts (empty values).
///
/// This test ensures that artifacts without prompts can still be stored
/// in the backend correctly.
#[test]
#[serial]
fn e2e_backend_storage_no_prompts() -> Result<()> {
    // Setup test storage
    let (storage_dir, storage_path) = setup_test_storage("no_prompts")?;
    let _cleanup = CleanupGuard;

    // Use the two-artifacts-no-prompts scenario
    let (backend, make_config) = load_example("scenarios/two-artifacts-no-prompts")?;

    let machine_name = make_config
        .nixos_map
        .keys()
        .next()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No machines found"))?;

    let (artifact_name, artifact_def) = find_first_artifact(&make_config, &machine_name)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    // Generate with empty prompts
    let prompt_values: PromptValues = BTreeMap::new();

    let result = generate_single_artifact(
        &machine_name,
        &artifact_def,
        &prompt_values,
        &backend,
        &make_config,
    )?;

    assert!(result.success, "Artifact without prompts should succeed");
    assert!(
        !result.generated_file_contents.is_empty(),
        "Should generate files"
    );

    // Verify artifact exists in backend
    verify_artifact_in_storage(&storage_path, &machine_name, &artifact_name)
        .with_context(|| "Artifact without prompts should be stored")?;

    // Clean up
    drop(storage_dir);

    Ok(())
}

// =============================================================================
// Guard struct for cleanup
// =============================================================================

/// RAII guard to ensure environment cleanup even on panic.
struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        cleanup_test_storage();
    }
}
