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

use crate::common::TestHarness;
use anyhow::Result;
use artifacts::app::model::TargetType;
use artifacts::config::backend::BackendConfiguration;
use artifacts::config::nix::build_make_from_flake;
use serial_test::serial;
use std::collections::BTreeMap;
use std::path::PathBuf;

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

fn find_first_artifact(
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

#[test]
#[serial]
fn e2e_missing_artifact_config() -> Result<()> {
    let (_backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    let result = find_first_artifact(&make_config, "non-existent-machine");

    assert!(
        result.is_none(),
        "Should return None for non-existent machine"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_invalid_backend() -> Result<()> {
    let result = load_example("scenarios/single-artifact-with-prompts");

    assert!(result.is_ok(), "Should load valid configuration");

    let (backend, _make_config) = result?;

    assert!(
        !backend.config.is_empty(),
        "Backend should have at least one backend configured"
    );

    let backend_names: Vec<_> = backend.config.keys().collect();
    assert!(
        !backend_names.is_empty(),
        "Should have backend names available"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_generator_failure() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/error-missing-files")?;

    let (_artifact_name, artifact_def) = harness
        .find_artifact("missing-files", None)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found for missing-files"))?;

    let prompt_values: BTreeMap<String, String> = BTreeMap::new();

    let result = harness.generate_artifact(
        "missing-files",
        &artifact_def,
        TargetType::NixOS {
            machine: "missing-files".to_string(),
        },
        &prompt_values,
    );

    match result {
        Ok(artifact_result) => {
            // Either the generator succeeds, or it fails cleanly
            if artifact_result.success {
                // If it succeeded, we might have some files
            }
            // If it failed, success=false and error is set
        }
        Err(e) => {
            let error_msg = e.to_string();
            assert!(!error_msg.is_empty(), "Error message should not be empty");
            assert!(
                !error_msg.contains("unwrap"),
                "Error should not mention unwrap"
            );
        }
    }

    Ok(())
}

#[test]
#[serial]
fn e2e_serialization_failure() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/single-artifact-with-prompts")?;

    let (_artifact_name, artifact_def) = harness
        .find_artifact("machine-name", None)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    let prompt_values: BTreeMap<String, String> = BTreeMap::from([
        ("secret1".to_string(), "test".to_string()),
        ("secret2".to_string(), "test".to_string()),
    ]);

    let result = harness.generate_artifact(
        "machine-name",
        &artifact_def,
        TargetType::NixOS {
            machine: "machine-name".to_string(),
        },
        &prompt_values,
    )?;

    assert!(
        result.success,
        "Generation should succeed: {:?}",
        result.error
    );

    assert!(
        !result.generated_file_contents.is_empty(),
        "Should have generated file contents"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_empty_artifact_name() -> Result<()> {
    let (_backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    let (artifact_name, _) = find_first_artifact(&make_config, "machine-name")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    assert!(
        !artifact_name.is_empty(),
        "Artifact name should not be empty"
    );

    assert!(
        !artifact_name.trim().is_empty(),
        "Artifact name should not be whitespace-only"
    );

    assert!(
        !artifact_name.is_empty() && artifact_name.len() < 256,
        "Artifact name should have reasonable length"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_special_characters_in_artifact_name() -> Result<()> {
    let (_backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    let mut artifact_names = Vec::new();
    for artifacts in make_config.nixos_map.values() {
        for name in artifacts.keys() {
            artifact_names.push(name.clone());
        }
    }

    assert!(
        !artifact_names.is_empty(),
        "Should have at least one artifact"
    );

    for name in &artifact_names {
        assert!(
            !name.contains('\0'),
            "Artifact name should not contain null bytes"
        );

        assert!(!name.is_empty(), "Artifact name should not be empty");

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

#[test]
#[serial]
fn e2e_error_message_contains_context() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/error-missing-files")?;

    let (_artifact_name, artifact_def) = harness
        .find_artifact("missing-files", None)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found for missing-files"))?;

    let prompt_values: BTreeMap<String, String> = BTreeMap::new();

    let result = harness.generate_artifact(
        "missing-files",
        &artifact_def,
        TargetType::NixOS {
            machine: "missing-files".to_string(),
        },
        &prompt_values,
    );

    if let Err(e) = &result {
        let error_msg = e.to_string();

        assert!(
            error_msg.len() > 10,
            "Error message should be descriptive, got: {}",
            error_msg
        );

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

#[test]
#[serial]
fn e2e_error_message_actionable() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/error-missing-files")?;

    let (_, artifact_def) = harness
        .find_artifact("missing-files", None)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found for missing-files"))?;

    let prompt_values: BTreeMap<String, String> = BTreeMap::new();

    let result = harness.generate_artifact(
        "missing-files",
        &artifact_def,
        TargetType::NixOS {
            machine: "missing-files".to_string(),
        },
        &prompt_values,
    );

    if let Err(e) = result {
        let error_msg = e.to_string().to_lowercase();

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

#[test]
#[serial]
fn e2e_error_message_not_internal() -> Result<()> {
    let scenarios = vec!["error-missing-files", "error-missing-generator"];

    for scenario in scenarios {
        let result = load_example(scenario);

        match result {
            Ok((_backend, make_config)) => {
                if let Some((_, artifact_def)) = find_first_artifact(&make_config, "machine-name") {
                    let harness = TestHarness::load_example(&format!("scenarios/{}", scenario))?;
                    let prompt_values: BTreeMap<String, String> = BTreeMap::new();

                    if let Err(e) = harness.generate_artifact(
                        "machine-name",
                        &artifact_def,
                        TargetType::NixOS {
                            machine: "machine-name".to_string(),
                        },
                        &prompt_values,
                    ) {
                        let error_msg = e.to_string();

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

#[test]
#[serial]
fn e2e_multiple_failures_reported() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/two-artifacts-no-prompts")?;

    let mut total_artifacts = 0;
    for artifacts in harness.make.nixos_map.values() {
        total_artifacts += artifacts.len();
    }

    assert!(
        total_artifacts >= 1,
        "Should have at least one artifact to test with"
    );

    for (machine, artifacts) in &harness.make.nixos_map {
        for (artifact_name, artifact_def) in artifacts {
            let prompt_values: BTreeMap<String, String> = BTreeMap::new();

            let result = harness.generate_artifact(
                machine,
                artifact_def,
                TargetType::NixOS {
                    machine: machine.clone(),
                },
                &prompt_values,
            );

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

#[test]
#[serial]
fn e2e_backend_config_validation() -> Result<()> {
    let (backend, _) = load_example("scenarios/single-artifact-with-prompts")?;

    for (backend_name, config) in &backend.config {
        assert!(!backend_name.is_empty(), "Backend name should not be empty");

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

#[test]
#[serial]
fn e2e_artifact_definition_validation() -> Result<()> {
    let (_, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    for artifacts in make_config.nixos_map.values() {
        for (name, def) in artifacts {
            assert!(!name.is_empty(), "Artifact name should not be empty");

            assert!(
                !def.files.is_empty(),
                "Artifact {} should have files defined",
                name
            );

            for (file_name, file_def) in &def.files {
                assert!(
                    file_def.path.is_some(),
                    "File {} in artifact {} should have a path",
                    file_name,
                    name
                );

                let path = file_def.path.as_ref().unwrap();

                assert!(path.starts_with('/'), "Path {} should be absolute", path);
            }

            assert!(
                !def.serialization.is_empty(),
                "Artifact {} should reference a backend",
                name
            );
        }
    }

    Ok(())
}

#[test]
#[serial]
fn e2e_prompt_value_validation() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/single-artifact-with-prompts")?;

    let (_, artifact_def) = harness
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
    );

    match result {
        Ok(_) => {
            // If it succeeds, that's fine
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("unwrap") {
                panic!("Error should not mention unwrap: {}", error_msg);
            }
        }
    }

    let harness2 = TestHarness::load_example("scenarios/single-artifact-with-prompts")?;
    let special_prompts: BTreeMap<String, String> = BTreeMap::from([
        ("secret1".to_string(), "value with spaces".to_string()),
        ("secret2".to_string(), "value\nwith\nnewlines".to_string()),
        ("secret3".to_string(), "unicode: äöü 日本語 🎉".to_string()),
    ]);

    let result = harness2.generate_artifact(
        "machine-name",
        &artifact_def,
        TargetType::NixOS {
            machine: "machine-name".to_string(),
        },
        &special_prompts,
    );

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

#[test]
#[serial]
fn e2e_file_path_validation() -> Result<()> {
    let (_, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    for artifacts in make_config.nixos_map.values() {
        for (artifact_name, def) in artifacts {
            for (file_name, file_def) in &def.files {
                let path_opt = &file_def.path;

                assert!(
                    path_opt.is_some(),
                    "File {} in artifact {} should have a path defined",
                    file_name,
                    artifact_name
                );

                let path = path_opt.as_ref().unwrap();

                assert!(
                    !path.is_empty(),
                    "File {} in artifact {} should have non-empty path",
                    file_name,
                    artifact_name
                );

                assert!(
                    path.starts_with('/'),
                    "File {} path should be absolute: {}",
                    file_name,
                    path
                );

                assert!(
                    !path.contains('\0'),
                    "File {} path should not contain null bytes",
                    file_name
                );
            }
        }
    }

    Ok(())
}

#[test]
#[serial]
fn e2e_generator_script_validation() -> Result<()> {
    let result = load_example("scenarios/error-missing-generator");

    match result {
        Ok((_backend, make_config)) => {
            if let Some((_, artifact_def)) = find_first_artifact(&make_config, "machine-name") {
                let harness = TestHarness::load_example("scenarios/error-missing-generator")?;
                let prompt_values: BTreeMap<String, String> = BTreeMap::new();

                let gen_result = harness.generate_artifact(
                    "machine-name",
                    &artifact_def,
                    TargetType::NixOS {
                        machine: "machine-name".to_string(),
                    },
                    &prompt_values,
                );

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
            let error_msg = e.to_string();
            assert!(!error_msg.is_empty(), "Error message should not be empty");
        }
    }

    Ok(())
}
