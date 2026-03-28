use crate::tui::model_state::ModelState;
use artifacts::app::model::{
    ArtifactEntry, ArtifactStatus, GeneratingState, GenerationStep, InputMode, ListEntry, LogEntry,
    LogLevel, LogStep, Model, PromptEntry, PromptState, Screen, SelectGeneratorState, SharedEntry,
    StepLogs, TargetType,
};
use artifacts::config::make::{
    ArtifactDef, FileDef, GeneratorInfo, GeneratorSource, PromptDef, TargetType as ConfigTargetType,
};
use artifacts::tui::views::{
    render_artifact_list, render_generator_selection, render_progress, render_prompt,
};
use insta::assert_snapshot;
use ratatui::{Terminal, backend::TestBackend};
use std::collections::BTreeMap;
use std::fmt;

// ============================================================================
// Snapshot types - capture input state alongside rendered output
// ============================================================================

struct ViewTestResult<S: fmt::Debug> {
    state: S,
    model: Option<ModelState>,
    rendered: String,
}

impl<S: fmt::Debug> fmt::Display for ViewTestResult<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write state with Debug formatting
        writeln!(f, "State:")?;
        writeln!(f, "{:#?}", self.state)?;

        // Include model state if present
        if let Some(ref model) = self.model {
            writeln!(f)?;
            writeln!(f, "Model:")?;
            writeln!(f, "{:#?}", model)?;
        }

        writeln!(f)?;
        writeln!(f, "Rendered:")?;
        // Write rendered output as-is (already has line-by-line format from TestBackend)
        write!(f, "{}", self.rendered)
    }
}

impl<S: fmt::Debug> ViewTestResult<S> {
    /// Add ModelState capture to this test result.
    #[allow(dead_code)]
    fn with_model(mut self, model: &Model) -> Self {
        self.model = Some(ModelState::from_model(model));
        self
    }
}

/// Snapshot representation of Model for artifact list views
#[allow(dead_code)]
#[derive(Debug)]
struct ArtifactListState {
    selected_index: usize,
    selected_log_step: &'static str,
    artifacts: Vec<ArtifactSnapshot>,
    error: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct ArtifactSnapshot {
    target: String,
    target_type: &'static str,
    name: String,
    status: String,
    has_logs: bool,
}

impl ArtifactListState {
    fn from_model(model: &Model) -> Self {
        Self {
            selected_index: model.selected_index,
            selected_log_step: model.selected_log_step.label(),
            artifacts: model
                .entries
                .iter()
                .map(|e| match e {
                    ListEntry::Single(single) => ArtifactSnapshot {
                        target: single.target_type.target_name().to_string(),
                        target_type: single.target_type.context_str(),
                        name: single.artifact.name.clone(),
                        status: format!("{:?}", single.status),
                        has_logs: !single.step_logs.check.is_empty()
                            || !single.step_logs.generate.is_empty()
                            || !single.step_logs.serialize.is_empty(),
                    },
                    ListEntry::Shared(shared) => ArtifactSnapshot {
                        target: "[shared]".to_string(),
                        target_type: "shared",
                        name: shared.info.artifact_name.clone(),
                        status: format!("{:?}", shared.status),
                        has_logs: !shared.step_logs.check.is_empty()
                            || !shared.step_logs.generate.is_empty()
                            || !shared.step_logs.serialize.is_empty(),
                    },
                })
                .collect(),
            error: model.error.clone(),
        }
    }
}

/// Snapshot representation of PromptState
#[allow(dead_code)]
#[derive(Debug)]
struct PromptSnapshot {
    artifact_name: String,
    prompt_index: usize,
    total_prompts: usize,
    current_prompt: Option<PromptSnapshotEntry>,
    input_mode: &'static str,
    buffer: String,
    collected_count: usize,
}

#[allow(dead_code)]
#[derive(Debug)]
struct PromptSnapshotEntry {
    name: String,
    description: Option<String>,
}

impl PromptSnapshot {
    fn from_state(state: &PromptState) -> Self {
        Self {
            artifact_name: state.artifact_name.clone(),
            prompt_index: state.current_prompt_index,
            total_prompts: state.prompts.len(),
            current_prompt: state.current_prompt().map(|p| PromptSnapshotEntry {
                name: p.name.clone(),
                description: p.description.clone(),
            }),
            input_mode: state.input_mode.label(),
            buffer: state.buffer.clone(),
            collected_count: state.collected.len(),
        }
    }
}

/// Snapshot representation of GeneratingState
#[allow(dead_code)]
#[derive(Debug)]
struct GeneratingSnapshot {
    artifact_name: String,
    step: &'static str,
    log_line_count: usize,
}

impl GeneratingSnapshot {
    fn from_state(state: &GeneratingState) -> Self {
        Self {
            artifact_name: state.artifact_name.clone(),
            step: match state.step {
                GenerationStep::CheckSerialization => "CheckSerialization",
                GenerationStep::RunningGenerator => "RunningGenerator",
                GenerationStep::Serializing => "Serializing",
            },
            log_line_count: state.log_lines.len(),
        }
    }
}

/// Snapshot representation of SelectGeneratorState
#[allow(dead_code)]
#[derive(Debug)]
struct GeneratorSelectionSnapshot {
    artifact_name: String,
    description: Option<String>,
    selected_index: usize,
    generators: Vec<GeneratorSnapshot>,
    prompts: Vec<PromptDefSnapshot>,
    nixos_targets: Vec<String>,
    home_targets: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct PromptDefSnapshot {
    name: String,
    description: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct GeneratorSnapshot {
    path: String,
    sources: Vec<SourceSnapshot>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct SourceSnapshot {
    target: String,
    target_type: &'static str,
}

impl GeneratorSelectionSnapshot {
    fn from_state(state: &SelectGeneratorState) -> Self {
        Self {
            artifact_name: state.artifact_name.clone(),
            description: state.description.clone(),
            selected_index: state.selected_index,
            generators: state
                .generators
                .iter()
                .map(|g| GeneratorSnapshot {
                    path: g.path.clone(),
                    sources: g
                        .sources
                        .iter()
                        .map(|s| SourceSnapshot {
                            target: s.target.clone(),
                            target_type: match s.target_type {
                                ConfigTargetType::Nixos => "nixos",
                                ConfigTargetType::HomeManager => "homemanager",
                            },
                        })
                        .collect(),
                })
                .collect(),
            prompts: state
                .prompts
                .iter()
                .map(|p| PromptDefSnapshot {
                    name: p.name.clone(),
                    description: p.description.clone(),
                })
                .collect(),
            nixos_targets: state.nixos_targets.clone(),
            home_targets: state.home_targets.clone(),
        }
    }
}

// ============================================================================
// Test helpers
// ============================================================================

fn make_test_artifact(name: &str, prompts: Vec<&str>) -> ArtifactDef {
    let mut prompt_map = BTreeMap::new();
    for p in prompts {
        prompt_map.insert(
            p.to_string(),
            PromptDef {
                name: p.to_string(),
                description: Some(format!("Enter the {} value", p)),
            },
        );
    }
    ArtifactDef {
        name: name.to_string(),
        description: None,
        shared: false,
        files: BTreeMap::from([(
            "test".to_string(),
            FileDef {
                name: "test".to_string(),
                path: Some("/test/path".to_string()),
                owner: None,
                group: None,
            },
        )]),
        prompts: prompt_map,
        generator: "/nix/store/xxx-gen".to_string(),
        serialization: "test-backend".to_string(),
    }
}

fn make_test_model() -> Model {
    let entry1 = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-one".to_string(),
        },
        artifact: make_test_artifact("ssh-key", vec!["passphrase"]),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };
    let entry2 = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-two".to_string(),
        },
        artifact: make_test_artifact("api-token", vec![]),
        status: ArtifactStatus::UpToDate,
        step_logs: StepLogs::default(),
    };
    let entry3 = ArtifactEntry {
        target_type: TargetType::HomeManager {
            username: "user@host".to_string(),
        },
        artifact: make_test_artifact("gpg-key", vec!["email", "name"]),
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
    };

    Model {
        screen: Screen::ArtifactList,
        entries: vec![
            ListEntry::Single(entry1),
            ListEntry::Single(entry2),
            ListEntry::Single(entry3),
        ],
        selected_index: 0,
        selected_log_step: LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    }
}

// Helper to update entry status and logs (for tests that modify entries directly)
fn set_entry_status(entries: &mut [ListEntry], index: usize, status: ArtifactStatus) {
    if let Some(entry) = entries.get_mut(index) {
        *entry.status_mut() = status;
    }
}

fn add_log_entry(entries: &mut [ListEntry], index: usize, step: LogStep, entry: LogEntry) {
    if let Some(entry_logs) = entries.get_mut(index) {
        entry_logs.step_logs_mut().get_mut(step).push(entry);
    }
}

// ============================================================================
// Artifact List View Tests
// ============================================================================

#[test]
fn test_artifact_list_initial() {
    let model = make_test_model();

    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: ArtifactListState::from_model(&model),
        model: Some(ModelState::from_model(&model)),
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_artifact_list_with_selection() {
    let mut model = make_test_model();
    model.selected_index = 1;
    model.selected_log_step = LogStep::Generate;

    // Add realistic logs for the selected artifact (api-token)
    add_log_entry(
        &mut model.entries,
        1,
        LogStep::Check,
        LogEntry {
            level: LogLevel::Success,
            message: "Already up to date".to_string(),
        },
    );
    add_log_entry(
        &mut model.entries,
        1,
        LogStep::Generate,
        LogEntry {
            level: LogLevel::Output,
            message: "Generating API token...".to_string(),
        },
    );
    add_log_entry(
        &mut model.entries,
        1,
        LogStep::Generate,
        LogEntry {
            level: LogLevel::Output,
            message: "Token generated successfully".to_string(),
        },
    );
    add_log_entry(
        &mut model.entries,
        1,
        LogStep::Generate,
        LogEntry {
            level: LogLevel::Success,
            message: "Generated 1 file(s)".to_string(),
        },
    );
    add_log_entry(
        &mut model.entries,
        1,
        LogStep::Serialize,
        LogEntry {
            level: LogLevel::Success,
            message: "Serialized to backend".to_string(),
        },
    );

    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: ArtifactListState::from_model(&model),
        model: Some(ModelState::from_model(&model)),
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_artifact_list_with_failed_status() {
    let mut model = make_test_model();

    // Update entries (field used for rendering)
    if let ListEntry::Single(ref mut entry) = model.entries[0] {
        entry.status = ArtifactStatus::Failed {
            error: "Generator script exited with code 1".to_string(),
            output: String::new(),
            retry_available: true,
        };
        entry.step_logs.check = vec![LogEntry {
            level: LogLevel::Info,
            message: "Artifact needs regeneration".to_string(),
        }];
        entry.step_logs.generate = vec![
            LogEntry {
                level: LogLevel::Output,
                message: "Generating SSH key pair...".to_string(),
            },
            LogEntry {
                level: LogLevel::Error,
                message: "ssh-keygen: permission denied".to_string(),
            },
            LogEntry {
                level: LogLevel::Error,
                message: "Generator failed: exit code 1".to_string(),
            },
        ];
    }
    model.selected_log_step = LogStep::Generate;

    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: ArtifactListState::from_model(&model),
        model: Some(ModelState::from_model(&model)),
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

// ============================================================================
// Prompt View Tests
// ============================================================================

#[test]
fn test_prompt_initial_line_mode() {
    let state = PromptState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        description: None,
        prompts: vec![PromptEntry {
            name: "passphrase".to_string(),
            description: Some("Enter the SSH key passphrase".to_string()),
        }],
        current_prompt_index: 0,
        input_mode: InputMode::Line,
        buffer: String::new(),
        collected: Default::default(),
    };

    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_prompt(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: PromptSnapshot::from_state(&state),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_prompt_with_input() {
    let state = PromptState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        description: None,
        prompts: vec![PromptEntry {
            name: "passphrase".to_string(),
            description: Some("Enter the SSH key passphrase".to_string()),
        }],
        current_prompt_index: 0,
        input_mode: InputMode::Line,
        buffer: "my-secret-pass".to_string(),
        collected: Default::default(),
    };

    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_prompt(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: PromptSnapshot::from_state(&state),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_prompt_hidden_mode() {
    let state = PromptState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        description: None,
        prompts: vec![PromptEntry {
            name: "passphrase".to_string(),
            description: Some("Enter the SSH key passphrase".to_string()),
        }],
        current_prompt_index: 0,
        input_mode: InputMode::Hidden,
        buffer: "secret123".to_string(),
        collected: Default::default(),
    };

    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_prompt(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: PromptSnapshot::from_state(&state),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_prompt_multiline_mode() {
    let state = PromptState {
        artifact_index: 0,
        artifact_name: "certificate".to_string(),
        description: None,
        prompts: vec![PromptEntry {
            name: "pem".to_string(),
            description: Some("Paste the certificate PEM content".to_string()),
        }],
        current_prompt_index: 0,
        input_mode: InputMode::Multiline,
        buffer: "-----BEGIN CERT-----".to_string(),
        collected: Default::default(),
    };

    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_prompt(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: PromptSnapshot::from_state(&state),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_prompt_second_of_three() {
    let mut collected = std::collections::HashMap::new();
    collected.insert("email".to_string(), "test@example.com".to_string());

    let state = PromptState {
        artifact_index: 0,
        artifact_name: "gpg-key".to_string(),
        description: None,
        prompts: vec![
            PromptEntry {
                name: "email".to_string(),
                description: Some("Enter email address".to_string()),
            },
            PromptEntry {
                name: "name".to_string(),
                description: Some("Enter full name".to_string()),
            },
            PromptEntry {
                name: "passphrase".to_string(),
                description: Some("Enter GPG passphrase".to_string()),
            },
        ],
        current_prompt_index: 1,
        input_mode: InputMode::Line,
        buffer: String::new(),
        collected,
    };

    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_prompt(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: PromptSnapshot::from_state(&state),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

// ============================================================================
// Progress View Tests
// ============================================================================

#[test]
fn test_progress_running_generator() {
    let state = GeneratingState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        step: GenerationStep::RunningGenerator,
        log_lines: vec![],
        exists: false,
    };

    let backend = TestBackend::new(60, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_progress(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: GeneratingSnapshot::from_state(&state),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_progress_serializing() {
    let state = GeneratingState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        step: GenerationStep::Serializing,
        log_lines: vec![
            "Generator completed successfully".to_string(),
            "Starting serialization...".to_string(),
        ],
        exists: true, // Test regeneration case
    };

    let backend = TestBackend::new(60, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_progress(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: GeneratingSnapshot::from_state(&state),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

// ============================================================================
// Multiple Machines Generate All View Tests
// ============================================================================

fn make_multiple_machines_artifact(name: &str) -> ArtifactDef {
    ArtifactDef {
        name: name.to_string(),
        description: None,
        shared: false,
        files: BTreeMap::from([(
            "test".to_string(),
            FileDef {
                name: "test".to_string(),
                path: Some("/test/path".to_string()),
                owner: None,
                group: None,
            },
        )]),
        prompts: BTreeMap::new(),
        generator: "/nix/store/xxx-gen".to_string(),
        serialization: "test-backend".to_string(),
    }
}

#[test]
fn test_multiple_machines_before_generate_all() {
    let entry1 = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-one".to_string(),
        },
        artifact: make_multiple_machines_artifact("artifact-one"),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };
    let entry2 = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-one".to_string(),
        },
        artifact: make_multiple_machines_artifact("artifact-two"),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };
    let entry3 = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-two".to_string(),
        },
        artifact: make_multiple_machines_artifact("artifact-one"),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };
    let entry4 = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-two".to_string(),
        },
        artifact: make_multiple_machines_artifact("artifact-two"),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };

    let model = Model {
        screen: Screen::ArtifactList,
        entries: vec![
            ListEntry::Single(entry1),
            ListEntry::Single(entry2),
            ListEntry::Single(entry3),
            ListEntry::Single(entry4),
        ],
        selected_index: 0,
        selected_log_step: LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    let backend = TestBackend::new(70, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: ArtifactListState::from_model(&model),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_multiple_machines_after_generate_all() {
    let entry1 = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-one".to_string(),
        },
        artifact: make_multiple_machines_artifact("artifact-one"),
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
    };
    let entry2 = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-one".to_string(),
        },
        artifact: make_multiple_machines_artifact("artifact-two"),
        status: ArtifactStatus::UpToDate,
        step_logs: StepLogs::default(),
    };
    let entry3 = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-two".to_string(),
        },
        artifact: make_multiple_machines_artifact("artifact-one"),
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
    };
    let entry4 = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-two".to_string(),
        },
        artifact: make_multiple_machines_artifact("artifact-two"),
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
    };

    let model = Model {
        screen: Screen::ArtifactList,
        entries: vec![
            ListEntry::Single(entry1),
            ListEntry::Single(entry2),
            ListEntry::Single(entry3),
            ListEntry::Single(entry4),
        ],
        selected_index: 0,
        selected_log_step: LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    let backend = TestBackend::new(70, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: ArtifactListState::from_model(&model),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_artifact_list_with_shared_artifacts() {
    use artifacts::config::make::SharedArtifactInfo;

    let single_entry = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-one".to_string(),
        },
        artifact: make_test_artifact("local-secret", vec![]),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };

    let shared_entry = SharedEntry {
        info: SharedArtifactInfo {
            artifact_name: "shared-secret".to_string(),
            description: None,
            generators: vec![],
            nixos_targets: vec!["machine-one".to_string(), "machine-two".to_string()],
            home_targets: vec![],
            backend_name: "test".to_string(),
            prompts: std::collections::BTreeMap::new(),
            files: std::collections::BTreeMap::new(),
            error: None,
        },
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
        selected_generator: None,
    };

    let model = Model {
        screen: Screen::ArtifactList,

        entries: vec![
            ListEntry::Shared(shared_entry), // Shared artifacts sorted first
            ListEntry::Single(single_entry),
        ],
        selected_index: 0,
        selected_log_step: LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: ArtifactListState::from_model(&model),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

// Helper function to create a shared entry with specific status
fn make_shared_entry_with_status(status: ArtifactStatus) -> SharedEntry {
    use artifacts::config::make::SharedArtifactInfo;

    SharedEntry {
        info: SharedArtifactInfo {
            artifact_name: "shared-secret".to_string(),
            description: None,
            generators: vec![],
            nixos_targets: vec!["machine-one".to_string(), "machine-two".to_string()],
            home_targets: vec![],
            backend_name: "test".to_string(),
            prompts: std::collections::BTreeMap::new(),
            files: std::collections::BTreeMap::new(),
            error: None,
        },
        status,
        step_logs: StepLogs::default(),
        selected_generator: None,
    }
}

#[test]
fn test_shared_artifact_pending_status() {
    let shared_entry = make_shared_entry_with_status(ArtifactStatus::Pending);

    let model = Model {
        screen: Screen::ArtifactList,

        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: ArtifactListState::from_model(&model),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_shared_artifact_needs_generation_status() {
    let shared_entry = make_shared_entry_with_status(ArtifactStatus::NeedsGeneration);

    let model = Model {
        screen: Screen::ArtifactList,

        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: ArtifactListState::from_model(&model),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_shared_artifact_up_to_date_status() {
    let shared_entry = make_shared_entry_with_status(ArtifactStatus::UpToDate);

    let model = Model {
        screen: Screen::ArtifactList,

        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: ArtifactListState::from_model(&model),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_shared_artifact_failed_runtime_error() {
    let mut shared_entry = make_shared_entry_with_status(ArtifactStatus::Failed {
        error: "Generator script exited with code 1".to_string(),
        output: "Error: permission denied".to_string(),
        retry_available: true,
    });
    // Add some check logs
    shared_entry.step_logs.check = vec![
        LogEntry {
            level: LogLevel::Info,
            message: "Checking if generation is needed...".to_string(),
        },
        LogEntry {
            level: LogLevel::Success,
            message: "Artifact needs regeneration".to_string(),
        },
    ];

    let model = Model {
        screen: Screen::ArtifactList,

        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: LogStep::Check,
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    let backend = TestBackend::new(70, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: ArtifactListState::from_model(&model),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_shared_artifact_failed_config_error() {
    let mut shared_entry = make_shared_entry_with_status(ArtifactStatus::Failed {
        error: "File definition mismatch: 'id_rsa' in machine-one but 'id_ed25519' in machine-two"
            .to_string(),
        output: String::new(),
        retry_available: false,
    });
    // Add check logs
    shared_entry.step_logs.check = vec![LogEntry {
        level: LogLevel::Error,
        message: "Validation failed: File definition mismatch".to_string(),
    }];

    let model = Model {
        screen: Screen::ArtifactList,

        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: LogStep::Check,
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    let backend = TestBackend::new(70, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: ArtifactListState::from_model(&model),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

// ============================================================================
// Generator Selection View Tests
// ============================================================================

#[test]
fn test_generator_selection_single_generator() {
    let state = SelectGeneratorState {
        artifact_index: 0,
        artifact_name: "shared-ssh-key".to_string(),
        description: None,
        generators: vec![GeneratorInfo {
            path: "/nix/store/xxx-gen-ssh".to_string(),
            sources: vec![
                GeneratorSource {
                    target: "machine-one".to_string(),
                    target_type: ConfigTargetType::Nixos,
                },
                GeneratorSource {
                    target: "machine-two".to_string(),
                    target_type: ConfigTargetType::Nixos,
                },
            ],
        }],
        selected_index: 0,
        prompts: vec![],
        nixos_targets: vec!["machine-one".to_string(), "machine-two".to_string()],
        home_targets: vec![],
    };

    let backend = TestBackend::new(70, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_generator_selection(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: GeneratorSelectionSnapshot::from_state(&state),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_generator_selection_multiple_generators() {
    let state = SelectGeneratorState {
        artifact_index: 0,
        artifact_name: "shared-api-key".to_string(),
        description: None,
        generators: vec![
            GeneratorInfo {
                path: "/nix/store/xxx-gen-prod".to_string(),
                sources: vec![GeneratorSource {
                    target: "prod-server".to_string(),
                    target_type: ConfigTargetType::Nixos,
                }],
            },
            GeneratorInfo {
                path: "/nix/store/yyy-gen-dev".to_string(),
                sources: vec![
                    GeneratorSource {
                        target: "dev-machine".to_string(),
                        target_type: ConfigTargetType::Nixos,
                    },
                    GeneratorSource {
                        target: "alice@workstation".to_string(),
                        target_type: ConfigTargetType::HomeManager,
                    },
                ],
            },
        ],
        selected_index: 0,
        prompts: vec![],
        nixos_targets: vec!["prod-server".to_string(), "dev-machine".to_string()],
        home_targets: vec!["alice@workstation".to_string()],
    };

    let backend = TestBackend::new(70, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_generator_selection(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: GeneratorSelectionSnapshot::from_state(&state),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_generator_selection_second_selected() {
    let state = SelectGeneratorState {
        artifact_index: 0,
        artifact_name: "shared-api-key".to_string(),
        description: None,
        generators: vec![
            GeneratorInfo {
                path: "/nix/store/xxx-gen-prod".to_string(),
                sources: vec![GeneratorSource {
                    target: "prod-server".to_string(),
                    target_type: ConfigTargetType::Nixos,
                }],
            },
            GeneratorInfo {
                path: "/nix/store/yyy-gen-dev".to_string(),
                sources: vec![GeneratorSource {
                    target: "dev-machine".to_string(),
                    target_type: ConfigTargetType::Nixos,
                }],
            },
        ],
        selected_index: 1,
        prompts: vec![],
        nixos_targets: vec!["prod-server".to_string(), "dev-machine".to_string()],
        home_targets: vec![],
    };

    let backend = TestBackend::new(70, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_generator_selection(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: GeneratorSelectionSnapshot::from_state(&state),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_generator_selection_mixed_source_types() {
    // Test: One generator with both NixOS and home-manager sources
    let state = SelectGeneratorState {
        artifact_index: 0,
        artifact_name: "shared-cert".to_string(),
        description: None,
        generators: vec![GeneratorInfo {
            path: "/nix/store/mixed-gen".to_string(),
            sources: vec![
                GeneratorSource {
                    target: "server-1".to_string(),
                    target_type: ConfigTargetType::Nixos,
                },
                GeneratorSource {
                    target: "server-2".to_string(),
                    target_type: ConfigTargetType::Nixos,
                },
                GeneratorSource {
                    target: "alice@laptop".to_string(),
                    target_type: ConfigTargetType::HomeManager,
                },
                GeneratorSource {
                    target: "bob@desktop".to_string(),
                    target_type: ConfigTargetType::HomeManager,
                },
            ],
        }],
        selected_index: 0,
        prompts: vec![],
        nixos_targets: vec!["server-1".to_string(), "server-2".to_string()],
        home_targets: vec!["alice@laptop".to_string(), "bob@desktop".to_string()],
    };

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_generator_selection(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: GeneratorSelectionSnapshot::from_state(&state),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_generator_selection_singular_vs_plural() {
    // Test: Single NixOS machine and single home-manager user (singular labels)
    let state = SelectGeneratorState {
        artifact_index: 0,
        artifact_name: "single-source-test".to_string(),
        description: None,
        generators: vec![
            GeneratorInfo {
                path: "/nix/store/single-nixos".to_string(),
                sources: vec![GeneratorSource {
                    target: "server-1".to_string(),
                    target_type: ConfigTargetType::Nixos,
                }],
            },
            GeneratorInfo {
                path: "/nix/store/single-home".to_string(),
                sources: vec![GeneratorSource {
                    target: "alice@laptop".to_string(),
                    target_type: ConfigTargetType::HomeManager,
                }],
            },
        ],
        selected_index: 0,
        prompts: vec![],
        nixos_targets: vec!["server-1".to_string()],
        home_targets: vec!["alice@laptop".to_string()],
    };

    let backend = TestBackend::new(80, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_generator_selection(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: GeneratorSelectionSnapshot::from_state(&state),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_generator_selection_many_sources() {
    // Test: Generator with many sources to verify layout
    let state = SelectGeneratorState {
        artifact_index: 0,
        artifact_name: "widely-used".to_string(),
        description: None,
        generators: vec![GeneratorInfo {
            path: "/nix/store/shared-gen".to_string(),
            sources: vec![
                GeneratorSource {
                    target: "server-1".to_string(),
                    target_type: ConfigTargetType::Nixos,
                },
                GeneratorSource {
                    target: "server-2".to_string(),
                    target_type: ConfigTargetType::Nixos,
                },
                GeneratorSource {
                    target: "server-3".to_string(),
                    target_type: ConfigTargetType::Nixos,
                },
                GeneratorSource {
                    target: "server-4".to_string(),
                    target_type: ConfigTargetType::Nixos,
                },
                GeneratorSource {
                    target: "server-5".to_string(),
                    target_type: ConfigTargetType::Nixos,
                },
            ],
        }],
        selected_index: 0,
        prompts: vec![],
        nixos_targets: vec![
            "server-1".to_string(),
            "server-2".to_string(),
            "server-3".to_string(),
            "server-4".to_string(),
            "server-5".to_string(),
        ],
        home_targets: vec![],
    };

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_generator_selection(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: GeneratorSelectionSnapshot::from_state(&state),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_generator_selection_multiple_with_mixed_sources() {
    // Test: Multiple generators, each with different source type mixes
    let state = SelectGeneratorState {
        artifact_index: 0,
        artifact_name: "complex-shared".to_string(),
        description: None,
        generators: vec![
            GeneratorInfo {
                path: "/nix/store/gen-prod".to_string(),
                sources: vec![
                    GeneratorSource {
                        target: "prod-1".to_string(),
                        target_type: ConfigTargetType::Nixos,
                    },
                    GeneratorSource {
                        target: "prod-2".to_string(),
                        target_type: ConfigTargetType::Nixos,
                    },
                ],
            },
            GeneratorInfo {
                path: "/nix/store/gen-dev".to_string(),
                sources: vec![
                    GeneratorSource {
                        target: "dev-1".to_string(),
                        target_type: ConfigTargetType::Nixos,
                    },
                    GeneratorSource {
                        target: "alice@dev".to_string(),
                        target_type: ConfigTargetType::HomeManager,
                    },
                    GeneratorSource {
                        target: "bob@dev".to_string(),
                        target_type: ConfigTargetType::HomeManager,
                    },
                ],
            },
            GeneratorInfo {
                path: "/nix/store/gen-personal".to_string(),
                sources: vec![GeneratorSource {
                    target: "charlie@home".to_string(),
                    target_type: ConfigTargetType::HomeManager,
                }],
            },
        ],
        selected_index: 1,
        prompts: vec![],
        nixos_targets: vec![
            "prod-1".to_string(),
            "prod-2".to_string(),
            "dev-1".to_string(),
        ],
        home_targets: vec![
            "alice@dev".to_string(),
            "bob@dev".to_string(),
            "charlie@home".to_string(),
        ],
    };

    let backend = TestBackend::new(80, 25);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_generator_selection(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: GeneratorSelectionSnapshot::from_state(&state),
        model: None,
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

// ============================================================================
// Model Tests - demonstrating Elm Architecture pattern
// ============================================================================

mod model_tests {
    use super::*;
    use artifacts::app::message::{KeyEvent, Message, ScriptOutput};
    use artifacts::app::model::{LogStep, Screen};
    use artifacts::app::update::update;

    /// State capture after running an event sequence
    #[allow(dead_code)]
    #[derive(Debug)]
    struct StateCapture {
        step_index: usize,
        message: String,
        model_state: ModelState,
        rendered: String,
    }

    impl fmt::Display for StateCapture {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            writeln!(f, "Step {}:", self.step_index)?;
            writeln!(f, "Message: {}", self.message)?;
            writeln!(f)?;
            writeln!(f, "Model:")?;
            writeln!(f, "{:#?}", self.model_state)?;
            writeln!(f)?;
            writeln!(f, "Rendered:")?;
            write!(f, "{}", self.rendered)
        }
    }

    /// Wrapper for Vec<StateCapture> to implement Display
    struct StateCaptures(Vec<StateCapture>);

    impl fmt::Display for StateCaptures {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for (i, capture) in self.0.iter().enumerate() {
                if i > 0 {
                    writeln!(f)?;
                }
                write!(f, "{}", capture)?;
            }
            Ok(())
        }
    }

    /// Run an event sequence and capture Model state and rendered view at each step
    fn run_event_sequence(mut model: Model, events: Vec<Message>) -> Vec<StateCapture> {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut captures = Vec::new();

        for (step_index, msg) in events.iter().enumerate() {
            // Capture state BEFORE processing (shows what state produced the view)
            let model_state = ModelState::from_model(&model);

            // Apply the message via pure update function
            let (new_model, _effect) = update(model, msg.clone());
            model = new_model;

            // Render the view based on new state
            terminal
                .draw(|f| match &model.screen {
                    Screen::ArtifactList => render_artifact_list(f, &model, f.area()),
                    Screen::Prompt(state) => render_prompt(f, state, f.area()),
                    _ => {}
                })
                .unwrap();

            let rendered = terminal.backend().to_string();

            // Create message description for documentation
            let message = format!("{:?}", msg);

            captures.push(StateCapture {
                step_index,
                message,
                model_state,
                rendered,
            });
        }

        captures
    }

    /// Test helper for creating Model with specific entry statuses
    fn make_model_with_statuses(statuses: Vec<ArtifactStatus>) -> Model {
        let entries: Vec<ListEntry> = statuses
            .into_iter()
            .enumerate()
            .map(|(i, status)| {
                let machine = format!("machine-{}", i + 1);
                ListEntry::Single(ArtifactEntry {
                    target_type: TargetType::NixOS {
                        machine: machine.clone(),
                    },
                    artifact: make_test_artifact(&format!("artifact-{}", i + 1), vec![]),
                    status,
                    step_logs: StepLogs::default(),
                })
            })
            .collect();

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

    #[test]
    fn test_navigate_down_updates_selection() {
        let model = make_test_model();
        let events = vec![Message::Key(KeyEvent::char('j'))];

        let captures = run_event_sequence(model, events);

        assert_snapshot!(StateCaptures(captures).to_string());
    }

    #[test]
    fn test_navigate_up_updates_selection() {
        let mut model = make_test_model();
        model.selected_index = 2; // Start at last item

        let events = vec![Message::Key(KeyEvent::char('k'))];

        let captures = run_event_sequence(model, events);

        assert_snapshot!(StateCaptures(captures).to_string());
    }

    #[test]
    fn test_navigation_sequence_j_k_j() {
        let model = make_test_model();
        let events = vec![
            Message::Key(KeyEvent::char('j')),
            Message::Key(KeyEvent::char('j')),
            Message::Key(KeyEvent::char('k')),
        ];

        let captures = run_event_sequence(model, events);

        assert_snapshot!(StateCaptures(captures).to_string());
    }

    #[test]
    fn test_artifact_list_with_failed_status() {
        let mut model = make_test_model();
        set_entry_status(
            &mut model.entries,
            0,
            ArtifactStatus::Failed {
                error: "Generator failed with exit code 1".to_string(),
                output: "Error: Missing required prompt value".to_string(),
                retry_available: true,
            },
        );

        // Add logs for the failed artifact
        add_log_entry(
            &mut model.entries,
            0,
            LogStep::Check,
            LogEntry {
                level: LogLevel::Success,
                message: "Needs generation".to_string(),
            },
        );
        add_log_entry(
            &mut model.entries,
            0,
            LogStep::Generate,
            LogEntry {
                level: LogLevel::Output,
                message: "Starting generator...".to_string(),
            },
        );
        add_log_entry(
            &mut model.entries,
            0,
            LogStep::Generate,
            LogEntry {
                level: LogLevel::Error,
                message: "Error: Missing required prompt value".to_string(),
            },
        );

        // Navigate to see logs
        let events = vec![Message::Key(KeyEvent::enter())];

        let captures = run_event_sequence(model, events);

        assert_snapshot!(StateCaptures(captures).to_string());
    }

    #[test]
    fn test_enter_opens_prompt_screen() {
        let mut model = make_test_model();
        // Set first artifact to NeedsGeneration so it can be generated
        if let ListEntry::Single(ref mut entry) = model.entries[0] {
            entry.status = ArtifactStatus::NeedsGeneration;
        }

        let events = vec![Message::Key(KeyEvent::enter())];

        let captures = run_event_sequence(model, events);

        assert_snapshot!(StateCaptures(captures).to_string());
    }

    #[test]
    fn test_esc_returns_to_list() {
        let mut model = make_test_model();
        // Set first artifact to NeedsGeneration
        if let ListEntry::Single(ref mut entry) = model.entries[0] {
            entry.status = ArtifactStatus::NeedsGeneration;
        }

        let events = vec![
            Message::Key(KeyEvent::enter()),
            Message::Key(KeyEvent::esc()),
        ];

        let captures = run_event_sequence(model, events);

        assert_snapshot!(StateCaptures(captures).to_string());
    }

    #[test]
    fn test_status_pending_to_needs_generation() {
        let model = make_model_with_statuses(vec![ArtifactStatus::Pending]);

        let events = vec![Message::CheckSerializationResult {
            artifact_index: 0,
            status: ArtifactStatus::NeedsGeneration,
            result: Ok(ScriptOutput::default()),
        }];

        let captures = run_event_sequence(model, events);

        assert_snapshot!(StateCaptures(captures).to_string());
    }

    #[test]
    fn test_status_up_to_date() {
        let model = make_model_with_statuses(vec![ArtifactStatus::Pending]);

        let events = vec![Message::CheckSerializationResult {
            artifact_index: 0,
            status: ArtifactStatus::UpToDate,
            result: Ok(ScriptOutput::default()),
        }];

        let captures = run_event_sequence(model, events);

        assert_snapshot!(StateCaptures(captures).to_string());
    }

    #[test]
    fn test_mixed_status_artifacts() {
        let model = make_model_with_statuses(vec![
            ArtifactStatus::UpToDate,
            ArtifactStatus::NeedsGeneration,
            ArtifactStatus::Pending,
        ]);

        // Capture initial state with no events
        let events = vec![];
        let captures = run_event_sequence(model, events);

        assert_snapshot!(StateCaptures(captures).to_string());
    }
}
