use crate::app::model::{
    ArtifactEntry, ArtifactStatus, ListEntry, LogStep, Model, Screen, SharedEntry, StepLogs,
    TargetType, Warning,
};
use crate::config::backend::BackendConfiguration;
use crate::config::make::MakeConfiguration;

/// Build the initial Model from a MakeConfiguration.
/// This extracts all artifacts from both nixos and home configurations.
pub fn build_model(make: &MakeConfiguration) -> Model {
    let mut entries = Vec::new();

    // Get shared artifacts - these will be represented once in the list
    let shared_artifacts = make.get_shared_artifacts();

    // Track which artifact names are shared so we skip individual instances
    let shared_names: std::collections::HashSet<&str> =
        shared_artifacts.keys().map(|s| s.as_str()).collect();

    // Add NixOS machine artifacts (skip shared ones)
    for (machine, machine_artifacts) in &make.nixos_map {
        for artifact in machine_artifacts.values() {
            if !shared_names.contains(artifact.name.as_str()) {
                let entry = ArtifactEntry {
                    target_type: TargetType::NixOS { machine: machine.clone() },
                    artifact: artifact.clone(),
                    status: ArtifactStatus::Pending,
                    step_logs: StepLogs::default(),
                };
                entries.push(ListEntry::Single(entry));
            }
        }
    }

    // Add home-manager user artifacts (skip shared ones)
    for (user, user_artifacts) in &make.home_map {
        for artifact in user_artifacts.values() {
            if !shared_names.contains(artifact.name.as_str()) {
                let entry = ArtifactEntry {
                    target_type: TargetType::HomeManager { username: user.clone() },
                    artifact: artifact.clone(),
                    status: ArtifactStatus::Pending,
                    step_logs: StepLogs::default(),
                };
                entries.push(ListEntry::Single(entry));
            }
        }
    }

    // Add shared artifacts as SharedEntry
    for (_name, shared_info) in shared_artifacts {
        // Check if artifact has validation error
        let status = if let Some(ref error) = shared_info.error {
            ArtifactStatus::Failed {
                error: error.clone(),
                output: String::new(),
                retry_available: false, // Validation errors don't benefit from retry
            }
        } else {
            ArtifactStatus::Pending
        };

        entries.push(ListEntry::Shared(SharedEntry {
            target_type: TargetType::Shared {
                nixos_targets: shared_info.nixos_targets.clone(),
                home_targets: shared_info.home_targets.clone(),
            },
            info: shared_info,
            status,
            step_logs: StepLogs::default(),
            selected_generator: None,
        }));
    }

    // Sort entries: shared first (by name), then single by target/name
    entries.sort_by(|a, b| match (a, b) {
        (ListEntry::Shared(sa), ListEntry::Shared(sb)) => {
            sa.info.artifact_name.cmp(&sb.info.artifact_name)
        }
        (ListEntry::Shared(_), ListEntry::Single(_)) => std::cmp::Ordering::Less,
        (ListEntry::Single(_), ListEntry::Shared(_)) => std::cmp::Ordering::Greater,
        (ListEntry::Single(ea), ListEntry::Single(eb)) => {
            let a_target = ea.target_type.target_name().unwrap_or("shared");
            let b_target = eb.target_type.target_name().unwrap_or("shared");
            (a_target, &ea.artifact.name).cmp(&(b_target, &eb.artifact.name))
        }
    });

    Model {
        screen: Screen::ArtifactList,
        entries,
        selected_index: 0,
        selected_log_step: LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    }
}

/// Build a model with backend capability validation.
/// Adds warnings for any capability issues (e.g., shared artifacts with backends that don't support shared).
pub fn build_model_with_validation(
    make: &MakeConfiguration,
    backend: &BackendConfiguration,
) -> Model {
    let mut model = build_model(make);
    let mut warnings = Vec::new();

    // Check shared artifacts against backend capabilities
    for entry in &model.entries {
        if let ListEntry::Shared(shared) = entry {
            let backend_name = &shared.info.backend_name;
            if let Ok(backend_entry) = backend.get_backend(backend_name)
                && !backend_entry.supports_shared()
            {
                warnings.push(Warning {
                        artifact_name: shared.info.artifact_name.clone(),
                        message: format!(
                            "Backend '{}' does not support shared artifacts (missing shared_serialize script)",
                            backend_name
                        ),
                    });
            }
        }
    }

    model.warnings = warnings;
    model
}

/// Add capability validation warnings to an existing model.
/// This is useful when you've already built a model (e.g., filtered) and want to add validation.
pub fn validate_model_capabilities(model: &mut Model, backend: &BackendConfiguration) {
    let mut warnings = Vec::new();

    for entry in &model.entries {
        if let ListEntry::Shared(shared) = entry {
            let backend_name = &shared.info.backend_name;
            if let Ok(backend_entry) = backend.get_backend(backend_name)
                && !backend_entry.supports_shared()
            {
                warnings.push(Warning {
                        artifact_name: shared.info.artifact_name.clone(),
                        message: format!(
                            "Backend '{}' does not support shared artifacts (missing shared_serialize script)",
                            backend_name
                        ),
                    });
            }
        }
    }

    model.warnings = warnings;
}

/// Build a model with only specific artifacts (for filtered commands).
pub fn build_filtered_model(
    make: &MakeConfiguration,
    machines: &[String],
    home_users: &[String],
    artifact_names: &[String],
) -> Model {
    let mut model = build_model(make);

    // If no filters specified, return full model
    if machines.is_empty() && home_users.is_empty() && artifact_names.is_empty() {
        return model;
    }

    // Filter entries
    model.entries.retain(|entry| match entry {
        ListEntry::Single(single) => {
            let target_matches = match &single.target_type {
                TargetType::NixOS { machine } => {
                    if !machines.is_empty() {
                        machines.contains(machine)
                    } else {
                        home_users.is_empty()
                    }
                }
                TargetType::HomeManager { username } => {
                    if !home_users.is_empty() {
                        home_users.contains(username)
                    } else {
                        machines.is_empty()
                    }
                }
                TargetType::Shared { .. } => false, // Handled separately
            };

            let artifact_matches =
                artifact_names.is_empty() || artifact_names.contains(&single.artifact.name);

            target_matches && artifact_matches
        }
        ListEntry::Shared(shared) => {
            // Shared artifacts match if:
            // - artifact name matches (if filter specified), AND
            // - at least one of its targets matches the machine/home filters
            let artifact_matches =
                artifact_names.is_empty() || artifact_names.contains(&shared.info.artifact_name);

            if !artifact_matches {
                return false;
            }

            // If no target filters, include all shared artifacts
            if machines.is_empty() && home_users.is_empty() {
                return true;
            }

            // Check if any target matches
            let has_matching_nixos = !machines.is_empty()
                && shared
                    .info
                    .nixos_targets
                    .iter()
                    .any(|t| machines.contains(t));

            let has_matching_home = !home_users.is_empty()
                && shared
                    .info
                    .home_targets
                    .iter()
                    .any(|u| home_users.contains(u));

            has_matching_nixos || has_matching_home
        }
    });

    model
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::make::{ArtifactDef, FileDef, PromptDef};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn make_test_config() -> MakeConfiguration {
        let mut nixos_map = BTreeMap::new();
        let mut home_map = BTreeMap::new();

        // Add NixOS artifacts
        let mut machine_one_artifacts = BTreeMap::new();
        machine_one_artifacts.insert(
            "ssh-key".to_string(),
            ArtifactDef {
                name: "ssh-key".to_string(),
                description: None,
                shared: false,
                files: BTreeMap::from([(
                    "id_ed25519".to_string(),
                    FileDef {
                        name: "id_ed25519".to_string(),
                        path: Some("/etc/ssh/id_ed25519".to_string()),
                        owner: Some("root".to_string()),
                        group: Some("root".to_string()),
                    },
                )]),
                prompts: BTreeMap::from([(
                    "passphrase".to_string(),
                    PromptDef {
                        name: "passphrase".to_string(),
                        description: Some("Enter passphrase".to_string()),
                    },
                )]),
                generator: "/gen/ssh".to_string(),
                serialization: "agenix".to_string(),
            },
        );
        nixos_map.insert("machine-one".to_string(), machine_one_artifacts);

        let mut machine_two_artifacts = BTreeMap::new();
        machine_two_artifacts.insert(
            "api-token".to_string(),
            ArtifactDef {
                name: "api-token".to_string(),
                description: None,
                shared: false,
                files: BTreeMap::from([(
                    "token".to_string(),
                    FileDef {
                        name: "token".to_string(),
                        path: Some("/etc/api-token".to_string()),
                        owner: None,
                        group: None,
                    },
                )]),
                prompts: BTreeMap::new(),
                generator: "/gen/token".to_string(),
                serialization: "agenix".to_string(),
            },
        );
        nixos_map.insert("machine-two".to_string(), machine_two_artifacts);

        // Add home-manager artifacts
        let mut user_artifacts = BTreeMap::new();
        user_artifacts.insert(
            "gpg-key".to_string(),
            ArtifactDef {
                name: "gpg-key".to_string(),
                description: None,
                shared: false,
                files: BTreeMap::from([(
                    "key.gpg".to_string(),
                    FileDef {
                        name: "key.gpg".to_string(),
                        path: Some("~/.gnupg/key.gpg".to_string()),
                        owner: None,
                        group: None,
                    },
                )]),
                prompts: BTreeMap::new(),
                generator: "/gen/gpg".to_string(),
                serialization: "sops".to_string(),
            },
        );
        home_map.insert("alice@desktop".to_string(), user_artifacts);

        MakeConfiguration {
            nixos_map,
            home_map,
            nixos_config: BTreeMap::new(),
            home_config: BTreeMap::new(),
            make_base: PathBuf::from("/test"),
            make_json: PathBuf::from("/test/make.json"),
        }
    }

    #[test]
    fn test_build_model_includes_all_artifacts() {
        let config = make_test_config();
        let model = build_model(&config);

        assert_eq!(model.entries.len(), 3);
        assert_eq!(model.selected_index, 0);
        assert!(matches!(model.screen, Screen::ArtifactList));
    }

    #[test]
    fn test_build_model_sorts_by_target_and_name() {
        let config = make_test_config();
        let model = build_model(&config);

        // Should be sorted: alice@desktop/gpg-key, machine-one/ssh-key, machine-two/api-token
        assert_eq!(model.entries[0].target_type().target_name().unwrap(), "alice@desktop");
        assert_eq!(model.entries[1].target_type().target_name().unwrap(), "machine-one");
        assert_eq!(model.entries[2].target_type().target_name().unwrap(), "machine-two");
    }

    #[test]
    fn test_build_filtered_model_by_machine() {
        let config = make_test_config();
        let model = build_filtered_model(&config, &["machine-one".to_string()], &[], &[]);

        assert_eq!(model.entries.len(), 1);
        match &model.entries[0] {
            ListEntry::Single(entry) => assert_eq!(entry.artifact.name, "ssh-key"),
            _ => panic!("Expected Single entry"),
        }
    }

    #[test]
    fn test_build_filtered_model_by_home_user() {
        let config = make_test_config();
        let model = build_filtered_model(&config, &[], &["alice@desktop".to_string()], &[]);

        assert_eq!(model.entries.len(), 1);
        match &model.entries[0] {
            ListEntry::Single(entry) => assert_eq!(entry.artifact.name, "gpg-key"),
            _ => panic!("Expected Single entry"),
        }
    }

    #[test]
    fn test_build_filtered_model_by_artifact_name() {
        let config = make_test_config();
        let model = build_filtered_model(&config, &[], &[], &["ssh-key".to_string()]);

        assert_eq!(model.entries.len(), 1);
        assert_eq!(model.entries[0].target_type().target_name().unwrap(), "machine-one");
    }

    #[test]
    fn test_build_filtered_model_no_filters_returns_all() {
        let config = make_test_config();
        let model = build_filtered_model(&config, &[], &[], &[]);

        assert_eq!(model.entries.len(), 3);
    }

    fn make_shared_artifact(name: &str) -> ArtifactDef {
        ArtifactDef {
            name: name.to_string(),
            description: None,
            shared: true,
            files: BTreeMap::from([(
                "secret".to_string(),
                FileDef {
                    name: "secret".to_string(),
                    path: Some("/run/secrets/shared".to_string()),
                    owner: None,
                    group: None,
                },
            )]),
            prompts: BTreeMap::new(),
            generator: "/gen/shared".to_string(),
            serialization: "test".to_string(),
        }
    }

    fn make_test_config_with_shared() -> MakeConfiguration {
        let mut nixos_map = BTreeMap::new();
        let home_map = BTreeMap::new();

        // Machine one with shared artifact
        let mut machine_one_artifacts = BTreeMap::new();
        machine_one_artifacts.insert(
            "shared-secret".to_string(),
            make_shared_artifact("shared-secret"),
        );
        machine_one_artifacts.insert(
            "unique-one".to_string(),
            ArtifactDef {
                name: "unique-one".to_string(),
                description: None,
                shared: false,
                files: BTreeMap::new(),
                prompts: BTreeMap::new(),
                generator: "/gen/one".to_string(),
                serialization: "test".to_string(),
            },
        );
        nixos_map.insert("machine-one".to_string(), machine_one_artifacts);

        // Machine two with same shared artifact
        let mut machine_two_artifacts = BTreeMap::new();
        machine_two_artifacts.insert(
            "shared-secret".to_string(),
            make_shared_artifact("shared-secret"),
        );
        machine_two_artifacts.insert(
            "unique-two".to_string(),
            ArtifactDef {
                name: "unique-two".to_string(),
                description: None,
                shared: false,
                files: BTreeMap::new(),
                prompts: BTreeMap::new(),
                generator: "/gen/two".to_string(),
                serialization: "test".to_string(),
            },
        );
        nixos_map.insert("machine-two".to_string(), machine_two_artifacts);

        MakeConfiguration {
            nixos_map,
            home_map,
            nixos_config: BTreeMap::new(),
            home_config: BTreeMap::new(),
            make_base: PathBuf::from("/test"),
            make_json: PathBuf::from("/test/make.json"),
        }
    }

    #[test]
    fn test_build_model_with_shared_artifacts() {
        let config = make_test_config_with_shared();
        let model = build_model(&config);

        // entries should have 3 items: 1 shared + 2 unique
        assert_eq!(model.entries.len(), 3);

        // First should be the shared one (sorted to top)
        assert!(model.entries[0].is_shared());
        assert_eq!(model.entries[0].artifact_name(), "shared-secret");

        // Remaining should be single entries
        assert!(!model.entries[1].is_shared());
        assert!(!model.entries[2].is_shared());
    }

    #[test]
    fn test_shared_entry_has_targets() {
        let config = make_test_config_with_shared();
        let model = build_model(&config);

        // Find the shared entry
        let shared = model.entries.iter().find(|e| e.is_shared()).unwrap();

        if let ListEntry::Shared(entry) = shared {
            // Should reference both machines
            assert!(
                entry
                    .info
                    .nixos_targets
                    .contains(&"machine-one".to_string())
            );
            assert!(
                entry
                    .info
                    .nixos_targets
                    .contains(&"machine-two".to_string())
            );
            assert_eq!(entry.info.nixos_targets.len(), 2);
        } else {
            panic!("Expected SharedEntry");
        }
    }

    // === File Validation Tests ===

    fn make_test_config_with_mismatched_files() -> MakeConfiguration {
        let mut nixos_map = BTreeMap::new();
        let home_map = BTreeMap::new();

        // Machine one with shared artifact
        let mut machine_one_artifacts = BTreeMap::new();
        machine_one_artifacts.insert(
            "shared-secret".to_string(),
            ArtifactDef {
                name: "shared-secret".to_string(),
                description: None,
                shared: true,
                files: BTreeMap::from([(
                    "id_ed25519".to_string(),
                    FileDef {
                        name: "id_ed25519".to_string(),
                        path: Some("/run/secrets/id_ed25519".to_string()),
                        owner: None,
                        group: None,
                    },
                )]),
                prompts: BTreeMap::new(),
                generator: "/gen/shared".to_string(),
                serialization: "test".to_string(),
            },
        );
        nixos_map.insert("machine-one".to_string(), machine_one_artifacts);

        // Machine two with different file count
        let mut machine_two_artifacts = BTreeMap::new();
        machine_two_artifacts.insert(
            "shared-secret".to_string(),
            ArtifactDef {
                name: "shared-secret".to_string(),
                description: None,
                shared: true,
                files: BTreeMap::from([
                    (
                        "id_ed25519".to_string(),
                        FileDef {
                            name: "id_ed25519".to_string(),
                            path: Some("/run/secrets/id_ed25519".to_string()),
                            owner: None,
                            group: None,
                        },
                    ),
                    (
                        "id_ed25519.pub".to_string(),
                        FileDef {
                            name: "id_ed25519.pub".to_string(),
                            path: Some("/run/secrets/id_ed25519.pub".to_string()),
                            owner: None,
                            group: None,
                        },
                    ),
                ]),
                prompts: BTreeMap::new(),
                generator: "/gen/shared".to_string(),
                serialization: "test".to_string(),
            },
        );
        nixos_map.insert("machine-two".to_string(), machine_two_artifacts);

        MakeConfiguration {
            nixos_map,
            home_map,
            nixos_config: BTreeMap::new(),
            home_config: BTreeMap::new(),
            make_base: PathBuf::from("/test"),
            make_json: PathBuf::from("/test/make.json"),
        }
    }

    #[test]
    fn test_shared_artifact_with_validation_error_has_failed_status() {
        let config = make_test_config_with_mismatched_files();
        let model = build_model(&config);

        // Find the shared entry
        let shared = model
            .entries
            .iter()
            .find(|e| e.is_shared())
            .expect("Expected shared entry");

        if let ListEntry::Shared(entry) = shared {
            // Should have error in info
            assert!(
                entry.info.error.is_some(),
                "Shared artifact with mismatched files should have error"
            );

            // Status should be Failed
            match &entry.status {
                ArtifactStatus::Failed {
                    error,
                    retry_available,
                    ..
                } => {
                    assert!(
                        error.contains("File definition mismatch"),
                        "Error should mention file definition mismatch"
                    );
                    assert!(
                        !retry_available,
                        "Validation errors should have retry_available: false"
                    );
                }
                other => panic!(
                    "Expected Failed status, got {:?}",
                    other
                ),
            }
        } else {
            panic!("Expected SharedEntry");
        }
    }

    #[test]
    fn test_shared_artifact_with_matching_files_has_pending_status() {
        let config = make_test_config_with_shared();
        let model = build_model(&config);

        // Find the shared entry
        let shared = model
            .entries
            .iter()
            .find(|e| e.is_shared())
            .expect("Expected shared entry");

        if let ListEntry::Shared(entry) = shared {
            // Should have no error
            assert!(
                entry.info.error.is_none(),
                "Shared artifact with matching files should have no error"
            );

            // Status should be Pending (ready for normal processing)
            assert!(
                matches!(entry.status, ArtifactStatus::Pending),
                "Expected Pending status for valid shared artifact, got {:?}",
                entry.status
            );
        } else {
            panic!("Expected SharedEntry");
        }
    }

    #[test]
    fn test_shared_artifact_single_target_has_pending_status() {
        let mut config = make_test_config_with_shared();
        // Remove machine-two, leaving only one target
        config.nixos_map.remove("machine-two");

        let model = build_model(&config);

        // Find the shared entry
        let shared = model
            .entries
            .iter()
            .find(|e| e.is_shared())
            .expect("Expected shared entry");

        if let ListEntry::Shared(entry) = shared {
            // Single target shared artifacts don't need validation
            assert!(
                entry.info.error.is_none(),
                "Single target shared artifact should have no error"
            );

            // Status should be Pending
            assert!(
                matches!(entry.status, ArtifactStatus::Pending),
                "Expected Pending status for single-target shared artifact"
            );
        } else {
            panic!("Expected SharedEntry");
        }
    }
}
