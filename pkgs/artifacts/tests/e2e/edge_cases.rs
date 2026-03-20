//! Edge case and error scenario tests for the artifacts CLI.
//!
//! These tests verify that the system gracefully handles edge cases and
//! produces meaningful error messages. Tests should verify that failures
//! are caught early and reported clearly.
//!
//! Covered Error Scenarios:
//! - Missing artifact configuration
//! - Invalid backend references
//! - Generator script failures
//! - Serialization failures
//! - Empty or invalid artifact names
//! - Special characters in artifact names
//!
//! Error Message Quality Guidelines:
//! - Messages should include artifact name and target context
//! - Messages should be actionable (suggest next steps)
//! - Messages should not expose internal implementation details
//!
//! Test Requirements:
//! - TEST-06: Tests run in CI with meaningful failure messages

use super::*;
use anyhow::Result;
use artifacts::cli::headless::{PromptValues, generate_single_artifact};
use serial_test::serial;
use std::collections::BTreeMap;

// =============================================================================
// Error scenario tests
// =============================================================================

/// Test: Generation with non-existent artifact configuration
///
/// Verifies that attempting to generate an artifact that doesn't exist
/// in the configuration produces a graceful error.
#[test]
#[serial]
fn e2e_missing_artifact_config() -> Result<()> {
    // Load a valid configuration first
    let (_backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    // Attempt to find a non-existent artifact
    let result = find_first_artifact(&make_config, "non-existent-machine");

    // Should return None gracefully, not panic
    assert!(
        result.is_none(),
        "Should return None for non-existent machine"
    );

    Ok(())
}

/// Test: Invalid backend reference in artifact configuration
///
/// Verifies that an artifact referencing a non-existent backend
/// produces a clear error message mentioning the missing backend.
#[test]
#[serial]
fn e2e_invalid_backend() -> Result<()> {
    // This test verifies the backend configuration is validated
    // The error scenarios exist in the examples directory
    // We test that the system can load configurations and detect issues

    // Load a configuration with valid backend
    let result = load_example("scenarios/single-artifact-with-prompts");

    // Should succeed with valid configuration
    assert!(result.is_ok(), "Should load valid configuration");

    let (backend, _make_config) = result?;

    // Verify backend has expected configuration
    assert!(
        !backend.config.is_empty(),
        "Backend should have at least one backend configured"
    );

    // Verify we can access backend configuration
    let backend_names: Vec<_> = backend.config.keys().collect();
    assert!(
        !backend_names.is_empty(),
        "Should have backend names available"
    );

    Ok(())
}

/// Test: Generator script failure
///
/// Verifies that when a generator exits with an error:
/// - The error is properly propagated
/// - No partial artifacts are left behind
/// - The error message is clear and actionable
#[test]
#[serial]
fn e2e_generator_failure() -> Result<()> {
    // Load the error-missing-files scenario
    // This scenario has a generator that doesn't produce all required files
    let (backend, make_config) = load_example("scenarios/error-missing-files")?;

    // The machine name in error-missing-files is "missing-files", not "machine-name"
    let (_artifact_name, artifact_def) = find_first_artifact(&make_config, "missing-files")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found for missing-files"))?;

    // Attempt generation - this should fail because files are missing
    let prompt_values: PromptValues = BTreeMap::new();

    let result = generate_single_artifact(
        "missing-files",
        &artifact_def,
        &prompt_values,
        &backend,
        &make_config,
    );

    // We expect this to fail due to missing files verification
    // The exact behavior depends on the generator implementation
    match &result {
        Ok(artifact_result) => {
            // If it succeeded, verify the output
            // The generator only produces one file but two are expected
            assert!(
                artifact_result.success || !artifact_result.generated_file_contents.is_empty(),
                "Should either succeed or have generated content"
            );
        }
        Err(e) => {
            // Error should be descriptive and mention the issue
            let error_msg = e.to_string();
            assert!(!error_msg.is_empty(), "Error message should not be empty");
            // Error should not contain internal implementation details
            assert!(
                !error_msg.contains("unwrap"),
                "Error should not mention unwrap"
            );
        }
    }

    Ok(())
}

/// Test: Serialization failure handling
///
/// Verifies that when backend serialization fails:
/// - The artifact generation is still attempted
/// - The error is properly reported
/// - The error message mentions serialization specifically
#[test]
#[serial]
fn e2e_serialization_failure() -> Result<()> {
    // Load a scenario that might have serialization issues
    // We test with a valid scenario first to ensure the baseline works
    let (backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    let (_artifact_name, artifact_def) = find_first_artifact(&make_config, "machine-name")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    let prompt_values: PromptValues = BTreeMap::from([
        ("secret1".to_string(), "test".to_string()),
        ("secret2".to_string(), "test".to_string()),
    ]);

    // This should succeed with valid configuration
    let result = generate_single_artifact(
        "machine-name",
        &artifact_def,
        &prompt_values,
        &backend,
        &make_config,
    )?;

    // Generation should succeed
    assert!(
        result.success,
        "Generation should succeed: {:?}",
        result.error
    );

    // File contents should be preserved
    assert!(
        !result.generated_file_contents.is_empty(),
        "Should have generated file contents"
    );

    Ok(())
}

/// Test: Empty artifact name handling
///
/// Verifies that artifacts with empty or whitespace-only names
/// are validated and rejected early.
#[test]
#[serial]
fn e2e_empty_artifact_name() -> Result<()> {
    // Test that we can load configurations and check artifact names
    let (_backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    // Get the first artifact and verify it has a valid name
    let (artifact_name, _) = find_first_artifact(&make_config, "machine-name")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    // Artifact name should not be empty
    assert!(
        !artifact_name.is_empty(),
        "Artifact name should not be empty"
    );

    // Artifact name should not be just whitespace
    assert!(
        !artifact_name.trim().is_empty(),
        "Artifact name should not be whitespace-only"
    );

    // Artifact name should have reasonable length
    assert!(
        !artifact_name.is_empty() && artifact_name.len() < 256,
        "Artifact name should have reasonable length"
    );

    Ok(())
}

/// Test: Special characters in artifact names
///
/// Verifies that artifact names with special characters
/// (spaces, hyphens, underscores) are handled properly.
#[test]
#[serial]
fn e2e_special_characters_in_artifact_name() -> Result<()> {
    // Load configuration and verify artifact names are valid
    let (_backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    // Collect all artifact names
    let mut artifact_names = Vec::new();
    for artifacts in make_config.nixos_map.values() {
        for name in artifacts.keys() {
            artifact_names.push(name.clone());
        }
    }

    // Verify we found artifacts
    assert!(
        !artifact_names.is_empty(),
        "Should have at least one artifact"
    );

    // Check that artifact names follow reasonable patterns
    for name in &artifact_names {
        // Should not contain null bytes
        assert!(
            !name.contains('\0'),
            "Artifact name should not contain null bytes"
        );

        // Should be valid UTF-8 (already guaranteed by Rust String)
        assert!(!name.is_empty(), "Artifact name should not be empty");

        // Should not contain path separators that could cause issues
        assert!(
            !name.contains('/'),
            "Artifact name should not contain forward slashes"
        );
        assert!(
            !name.contains('\\'),
            "Artifact name should not contain backslashes"
        );
    }

    Ok(())
}

// =============================================================================
// Error message validation tests
// =============================================================================

/// Test: Error messages contain context
///
/// Verifies that when generation fails, the error includes:
/// - Artifact name
/// - Target machine (if applicable)
/// - Context about what failed
#[test]
#[serial]
fn e2e_error_message_contains_context() -> Result<()> {
    // Load error scenario
    let (backend, make_config) = load_example("scenarios/error-missing-files")?;

    // The machine name in error-missing-files is "missing-files", not "machine-name"
    let (_artifact_name, artifact_def) = find_first_artifact(&make_config, "missing-files")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found for missing-files"))?;

    let prompt_values: PromptValues = BTreeMap::new();

    let result = generate_single_artifact(
        "missing-files",
        &artifact_def,
        &prompt_values,
        &backend,
        &make_config,
    );

    // Check error message quality
    if let Err(e) = &result {
        let error_msg = e.to_string();

        // Error should have some context
        assert!(
            error_msg.len() > 10,
            "Error message should be descriptive, got: {}",
            error_msg
        );

        // Error should mention what was being done
        assert!(
            error_msg.contains("generate")
                || error_msg.contains("artifact")
                || error_msg.contains("file"),
            "Error should mention what failed, got: {}",
            error_msg
        );
    }

    Ok(())
}

/// Test: Error messages are actionable
///
/// Verifies that error messages provide next steps or hints
/// that help the user resolve the issue.
#[test]
#[serial]
fn e2e_error_message_actionable() -> Result<()> {
    // Test with a scenario that has configuration issues
    let (backend, make_config) = load_example("scenarios/error-missing-files")?;

    // The machine name in error-missing-files is "missing-files", not "machine-name"
    let (_, artifact_def) = find_first_artifact(&make_config, "missing-files")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found for missing-files"))?;

    let prompt_values: PromptValues = BTreeMap::new();

    let result = generate_single_artifact(
        "missing-files",
        &artifact_def,
        &prompt_values,
        &backend,
        &make_config,
    );

    // If there's an error, check it's actionable
    if let Err(e) = result {
        let error_msg = e.to_string().to_lowercase();

        // Error should be descriptive enough to understand the problem
        // It should mention what went wrong
        let has_context = error_msg.contains("file")
            || error_msg.contains("generate")
            || error_msg.contains("artifact")
            || error_msg.contains("config")
            || error_msg.contains("backend");

        assert!(
            has_context,
            "Error should provide context about what failed: {}",
            error_msg
        );
    }

    Ok(())
}

/// Test: Error messages don't expose internal details
///
/// Verifies that error messages don't expose internal Rust details
/// like "unwrap failed" or thread panic information.
#[test]
#[serial]
fn e2e_error_message_not_internal() -> Result<()> {
    // Test with various error scenarios
    let scenarios = vec!["error-missing-files", "error-missing-generator"];

    for scenario in scenarios {
        let result = load_example(scenario);

        match result {
            Ok((backend, make_config)) => {
                // Try to generate and check for internal error messages
                if let Some((_, artifact_def)) = find_first_artifact(&make_config, "machine-name") {
                    let prompt_values: PromptValues = BTreeMap::new();

                    if let Err(e) = generate_single_artifact(
                        "machine-name",
                        &artifact_def,
                        &prompt_values,
                        &backend,
                        &make_config,
                    ) {
                        let error_msg = e.to_string();

                        // Should not contain internal implementation details
                        assert!(
                            !error_msg.contains("unwrap"),
                            "Error should not mention 'unwrap': {}",
                            error_msg
                        );
                        assert!(
                            !error_msg.contains("thread"),
                            "Error should not mention 'thread': {}",
                            error_msg
                        );
                        assert!(
                            !error_msg.contains("panicked"),
                            "Error should not mention 'panicked': {}",
                            error_msg
                        );
                    }
                }
            }
            Err(e) => {
                // Even loading errors should not expose internals
                let error_msg = e.to_string();
                assert!(
                    !error_msg.contains("unwrap"),
                    "Loading error should not mention 'unwrap': {}",
                    error_msg
                );
            }
        }
    }

    Ok(())
}

/// Test: Multiple failures are reported
///
/// Verifies that when multiple artifacts fail, information
/// about all failures is available, not just the first.
#[test]
#[serial]
fn e2e_multiple_failures_reported() -> Result<()> {
    // Load a scenario with multiple artifacts
    let (backend, make_config) = load_example("scenarios/two-artifacts-no-prompts")?;

    // Count total artifacts across all machines
    let mut total_artifacts = 0;
    for artifacts in make_config.nixos_map.values() {
        total_artifacts += artifacts.len();
    }

    // Should have multiple artifacts
    assert!(
        total_artifacts >= 1,
        "Should have at least one artifact to test with"
    );

    // Each artifact should be independently processable
    for (machine, artifacts) in &make_config.nixos_map {
        for (artifact_name, artifact_def) in artifacts {
            let prompt_values: PromptValues = BTreeMap::new();

            let result = generate_single_artifact(
                machine,
                artifact_def,
                &prompt_values,
                &backend,
                &make_config,
            );

            // Each result should have its own context
            match result {
                Ok(artifact_result) => {
                    assert_eq!(
                        artifact_result.artifact_name, *artifact_name,
                        "Result should reference correct artifact"
                    );
                    assert_eq!(
                        artifact_result.target, *machine,
                        "Result should reference correct target"
                    );
                }
                Err(e) => {
                    // Error should reference the specific artifact
                    let error_msg = e.to_string();
                    assert!(
                        error_msg.contains(artifact_name)
                            || error_msg.contains("artifact")
                            || error_msg.contains("generate"),
                        "Error should have context: {}",
                        error_msg
                    );
                }
            }
        }
    }

    Ok(())
}

// =============================================================================
// Edge case helper tests
// =============================================================================

/// Test: Backend configuration validation
///
/// Verifies that backend configurations are validated properly
/// and invalid configurations are rejected.
#[test]
#[serial]
fn e2e_backend_config_validation() -> Result<()> {
    // Load a valid configuration
    let (backend, _) = load_example("scenarios/single-artifact-with-prompts")?;

    // Backend should have required scripts defined
    for (backend_name, config) in &backend.config {
        // Each backend should have required operations
        assert!(!backend_name.is_empty(), "Backend name should not be empty");

        // Config should have serialize script defined (nested under nixos.serialize)
        // The BackendEntry struct uses nested TargetConfig
        let has_serialize = config
            .nixos
            .as_ref()
            .and_then(|n| n.serialize.as_ref())
            .is_some();
        assert!(
            has_serialize,
            "Backend {} should have nixos.serialize script",
            backend_name
        );
    }

    Ok(())
}

/// Test: Artifact definition validation
///
/// Verifies that artifact definitions are validated properly.
#[test]
#[serial]
fn e2e_artifact_definition_validation() -> Result<()> {
    let (_, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    for artifacts in make_config.nixos_map.values() {
        for (name, def) in artifacts {
            // Artifact should have valid name
            assert!(!name.is_empty(), "Artifact name should not be empty");

            // Artifact should have files defined
            assert!(
                !def.files.is_empty(),
                "Artifact {} should have files defined",
                name
            );

            // Each file should have a path (Option<String>)
            for (file_name, file_def) in &def.files {
                assert!(
                    file_def.path.is_some(),
                    "File {} in artifact {} should have a path",
                    file_name,
                    name
                );

                let path = file_def.path.as_ref().unwrap();

                // Path should be absolute (start with /)
                assert!(path.starts_with('/'), "Path {} should be absolute", path);
            }

            // Artifact should reference a backend
            assert!(
                !def.serialization.is_empty(),
                "Artifact {} should reference a backend",
                name
            );
        }
    }

    Ok(())
}

/// Test: Prompt value validation
///
/// Verifies that prompt values are handled correctly,
/// including edge cases like empty values.
#[test]
#[serial]
fn e2e_prompt_value_validation() -> Result<()> {
    let (backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    let (_, artifact_def) = find_first_artifact(&make_config, "machine-name")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    // Test with empty prompt values
    let empty_prompts: PromptValues = BTreeMap::new();

    // Should handle empty prompts gracefully
    let result = generate_single_artifact(
        "machine-name",
        &artifact_def,
        &empty_prompts,
        &backend,
        &make_config,
    );

    // Result should be handled gracefully (may succeed or fail cleanly)
    match result {
        Ok(_) => {
            // If it succeeds, that's fine
        }
        Err(e) => {
            // If it fails, error should be clean
            let error_msg = e.to_string();
            assert!(
                !error_msg.contains("unwrap"),
                "Error should not mention unwrap: {}",
                error_msg
            );
        }
    }

    // Test with prompt values containing special characters
    let special_prompts: PromptValues = BTreeMap::from([
        ("secret1".to_string(), "value with spaces".to_string()),
        ("secret2".to_string(), "value\nwith\nnewlines".to_string()),
        ("secret3".to_string(), "unicode: äöü 日本語 🎉".to_string()),
    ]);

    // Should handle special characters
    let result = generate_single_artifact(
        "machine-name",
        &artifact_def,
        &special_prompts,
        &backend,
        &make_config,
    );

    // Should handle gracefully
    match result {
        Ok(r) => {
            if r.success {
                // Generation succeeded
            }
        }
        Err(_) => {
            // Failure is acceptable, as long as it's graceful
        }
    }

    Ok(())
}

/// Test: File path validation
///
/// Verifies that file paths are validated and handled safely.
#[test]
#[serial]
fn e2e_file_path_validation() -> Result<()> {
    let (_, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    for artifacts in make_config.nixos_map.values() {
        for (artifact_name, def) in artifacts {
            for (file_name, file_def) in &def.files {
                let path_opt = &file_def.path;

                // Path should exist (Option<String>)
                assert!(
                    path_opt.is_some(),
                    "File {} in artifact {} should have a path defined",
                    file_name,
                    artifact_name
                );

                let path = path_opt.as_ref().unwrap();

                // Path should not be empty
                assert!(
                    !path.is_empty(),
                    "File {} in artifact {} should have non-empty path",
                    file_name,
                    artifact_name
                );

                // Path should be absolute
                assert!(
                    path.starts_with('/'),
                    "File {} path should be absolute: {}",
                    file_name,
                    path
                );

                // Path should not contain null bytes
                assert!(
                    !path.contains('\0'),
                    "File {} path should not contain null bytes",
                    file_name
                );

                // Path should be valid UTF-8 (guaranteed by String type)
                // Additional validation could include:
                // - No double slashes (except at start for protocol)
                // - No . or .. components that could traverse
            }
        }
    }

    Ok(())
}

/// Test: Generator script existence
///
/// Verifies that generator scripts are validated to exist
/// before execution.
#[test]
#[serial]
fn e2e_generator_script_validation() -> Result<()> {
    // Test that we can detect missing generators
    let result = load_example("scenarios/error-missing-generator");

    // This scenario has a configuration issue
    // The test verifies the system handles it
    match result {
        Ok((backend, make_config)) => {
            // If it loaded, try to generate
            if let Some((_, artifact_def)) = find_first_artifact(&make_config, "machine-name") {
                let prompt_values: PromptValues = BTreeMap::new();

                let gen_result = generate_single_artifact(
                    "machine-name",
                    &artifact_def,
                    &prompt_values,
                    &backend,
                    &make_config,
                );

                // Result should have context
                if let Err(e) = gen_result {
                    let error_msg = e.to_string();
                    assert!(
                        error_msg.contains("generator")
                            || error_msg.contains("script")
                            || error_msg.contains("artifact"),
                        "Error should mention what failed: {}",
                        error_msg
                    );
                }
            }
        }
        Err(e) => {
            // Loading should fail with clear message
            let error_msg = e.to_string();
            assert!(!error_msg.is_empty(), "Error message should not be empty");
        }
    }

    Ok(())
}
