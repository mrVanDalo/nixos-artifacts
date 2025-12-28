use crate::app::model::{ArtifactEntry, ArtifactStatus, Model, Screen, TargetType};
use crate::config::make::MakeConfiguration;

/// Build the initial Model from a MakeConfiguration.
/// This extracts all artifacts from both nixos and home configurations.
pub fn build_model(make: &MakeConfiguration) -> Model {
    let mut artifacts = Vec::new();

    // Add NixOS machine artifacts
    for (machine, machine_artifacts) in &make.nixos_map {
        for artifact in machine_artifacts.values() {
            artifacts.push(ArtifactEntry {
                target: machine.clone(),
                target_type: TargetType::Nixos,
                artifact: artifact.clone(),
                status: ArtifactStatus::Pending,
            });
        }
    }

    // Add home-manager user artifacts
    for (user, user_artifacts) in &make.home_map {
        for artifact in user_artifacts.values() {
            artifacts.push(ArtifactEntry {
                target: user.clone(),
                target_type: TargetType::HomeManager,
                artifact: artifact.clone(),
                status: ArtifactStatus::Pending,
            });
        }
    }

    // Sort artifacts by target then name for consistent ordering
    artifacts.sort_by(|a, b| {
        (&a.target, &a.artifact.name).cmp(&(&b.target, &b.artifact.name))
    });

    Model {
        screen: Screen::ArtifactList,
        artifacts,
        selected_index: 0,
        error: None,
    }
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

    // Filter artifacts
    model.artifacts.retain(|entry| {
        // Target filtering: when a specific target type filter is provided,
        // only include entries of that type that match
        let target_matches = match entry.target_type {
            TargetType::Nixos => {
                if !machines.is_empty() {
                    machines.contains(&entry.target)
                } else {
                    // If only home_users is specified, exclude NixOS entries
                    home_users.is_empty()
                }
            }
            TargetType::HomeManager => {
                if !home_users.is_empty() {
                    home_users.contains(&entry.target)
                } else {
                    // If only machines is specified, exclude home entries
                    machines.is_empty()
                }
            }
        };

        let artifact_matches =
            artifact_names.is_empty() || artifact_names.contains(&entry.artifact.name);

        target_matches && artifact_matches
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

        assert_eq!(model.artifacts.len(), 3);
        assert_eq!(model.selected_index, 0);
        assert!(matches!(model.screen, Screen::ArtifactList));
    }

    #[test]
    fn test_build_model_sorts_by_target_and_name() {
        let config = make_test_config();
        let model = build_model(&config);

        // Should be sorted: alice@desktop/gpg-key, machine-one/ssh-key, machine-two/api-token
        assert_eq!(model.artifacts[0].target, "alice@desktop");
        assert_eq!(model.artifacts[1].target, "machine-one");
        assert_eq!(model.artifacts[2].target, "machine-two");
    }

    #[test]
    fn test_build_filtered_model_by_machine() {
        let config = make_test_config();
        let model = build_filtered_model(&config, &["machine-one".to_string()], &[], &[]);

        assert_eq!(model.artifacts.len(), 1);
        assert_eq!(model.artifacts[0].artifact.name, "ssh-key");
    }

    #[test]
    fn test_build_filtered_model_by_home_user() {
        let config = make_test_config();
        let model = build_filtered_model(&config, &[], &["alice@desktop".to_string()], &[]);

        assert_eq!(model.artifacts.len(), 1);
        assert_eq!(model.artifacts[0].artifact.name, "gpg-key");
    }

    #[test]
    fn test_build_filtered_model_by_artifact_name() {
        let config = make_test_config();
        let model = build_filtered_model(&config, &[], &[], &["ssh-key".to_string()]);

        assert_eq!(model.artifacts.len(), 1);
        assert_eq!(model.artifacts[0].target, "machine-one");
    }

    #[test]
    fn test_build_filtered_model_no_filters_returns_all() {
        let config = make_test_config();
        let model = build_filtered_model(&config, &[], &[], &[]);

        assert_eq!(model.artifacts.len(), 3);
    }
}
