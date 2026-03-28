//! Shared test infrastructure for capturing full Model state using the Debug trait.
//!
//! This module provides the `ModelState` struct for capturing and snapshotting
//! the complete application state in tests. It enables documentation of the
//! Elm Architecture pattern: inputs → Model transformations → View rendering.

use artifacts::app::model::{ListEntry, Model, Screen, TargetType};

/// Snapshot representation of Model for test state capture.
///
/// This struct captures the essential state of the Model for testing purposes,
/// using `#[derive(Debug)]` for automatic field capture in snapshots.
#[allow(dead_code)]
#[derive(Debug)]
pub struct ModelState {
    pub screen: &'static str,
    pub selected_index: usize,
    pub selected_log_step: &'static str,
    pub artifacts: Vec<ArtifactState>,
    pub error: Option<String>,
    pub warnings_count: usize,
}

/// Snapshot representation of an individual artifact in the list.
#[allow(dead_code)]
#[derive(Debug)]
pub struct ArtifactState {
    pub target: String,
    pub name: String,
    pub status: String,
    pub target_type: &'static str,
}

impl ModelState {
    /// Create a ModelState from a Model instance.
    ///
    /// This method extracts the essential state from a Model for snapshot testing,
    /// converting enums to string labels and normalizing status strings for
    /// environment-independent snapshots.
    pub fn from_model(model: &Model) -> Self {
        Self {
            screen: match &model.screen {
                Screen::ArtifactList => "ArtifactList",
                Screen::SelectGenerator(_) => "SelectGenerator",
                Screen::ConfirmRegenerate(_) => "ConfirmRegenerate",
                Screen::Prompt(_) => "Prompt",
                Screen::Generating(_) => "Generating",
                Screen::Done(_) => "Done",
                Screen::ChronologicalLog(_) => "ChronologicalLog",
            },
            selected_index: model.selected_index,
            selected_log_step: model.selected_log_step.label(),
            artifacts: model
                .entries
                .iter()
                .map(|entry| match entry {
                    ListEntry::Single(single) => ArtifactState {
                        target: single.target_type.target_name().to_string(),
                        name: single.artifact.name.clone(),
                        status: normalize_status(format!("{:?}", single.status)),
                        target_type: single.target_type.context_str(),
                    },
                    ListEntry::Shared(shared) => ArtifactState {
                        target: "[shared]".to_string(),
                        name: shared.info.artifact_name.clone(),
                        status: normalize_status(format!("{:?}", shared.status)),
                        target_type: "shared",
                    },
                })
                .collect(),
            error: model.error.clone(),
            warnings_count: model.warnings.len(),
        }
    }
}

/// Normalize status strings by replacing absolute paths with a placeholder.
///
/// This ensures snapshots are stable across different environments by
/// removing project-specific paths from status strings.
fn normalize_status(status: String) -> String {
    let project = project_root().display().to_string();
    status.replace(&project, "[PROJECT]")
}

/// Get the project root directory.
///
/// Returns the path to the project root (the directory containing Cargo.toml).
fn project_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use artifacts::app::model::{ArtifactEntry, ArtifactStatus, LogStep, StepLogs};
    use artifacts::config::make::{ArtifactDef, FileDef};
    use std::collections::BTreeMap;

    fn make_test_artifact(name: &str) -> ArtifactDef {
        let mut files = BTreeMap::new();
        files.insert(
            "test".to_string(),
            FileDef {
                name: "test".to_string(),
                path: Some("/test/path".to_string()),
                owner: None,
                group: None,
            },
        );

        ArtifactDef {
            name: name.to_string(),
            description: None,
            shared: false,
            files,
            prompts: BTreeMap::new(),
            generator: "/nix/store/xxx-gen".to_string(),
            serialization: "test".to_string(),
        }
    }

    #[test]
    fn test_model_state_from_empty_model() {
        let model = Model {
            screen: Screen::ArtifactList,
            entries: vec![],
            selected_index: 0,
            selected_log_step: LogStep::default(),
            error: None,
            warnings: vec![],
            tick_count: 0,
        };

        let state = ModelState::from_model(&model);

        assert_eq!(state.screen, "ArtifactList");
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.selected_log_step, "Check");
        assert!(state.artifacts.is_empty());
        assert!(state.error.is_none());
        assert_eq!(state.warnings_count, 0);
    }

    #[test]
    fn test_model_state_with_single_entry() {
        let entry = ArtifactEntry {
            target_type: TargetType::NixOS {
                machine: "machine-one".to_string(),
            },
            artifact: make_test_artifact("test-artifact"),
            status: ArtifactStatus::NeedsGeneration,
            step_logs: StepLogs::default(),
        };

        let model = Model {
            screen: Screen::Generating(artifacts::app::model::GeneratingState {
                artifact_index: 0,
                artifact_name: "test-artifact".to_string(),
                step: artifacts::app::model::GenerationStep::RunningGenerator,
                log_lines: vec![],
                exists: false,
            }),
            entries: vec![ListEntry::Single(entry)],
            selected_index: 0,
            selected_log_step: LogStep::Generate,
            error: None,
            warnings: vec![],
            tick_count: 0,
        };

        let state = ModelState::from_model(&model);

        assert_eq!(state.screen, "Generating");
        assert_eq!(state.selected_log_step, "Generate");
        assert_eq!(state.artifacts.len(), 1);
        assert_eq!(state.artifacts[0].target, "machine-one");
        assert_eq!(state.artifacts[0].name, "test-artifact");
        assert_eq!(state.artifacts[0].target_type, "nixos");
    }
}
