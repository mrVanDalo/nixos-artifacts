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
use ratatui::{backend::TestBackend, Terminal};
use std::collections::BTreeMap;
use std::fmt;

// ============================================================================
// Snapshot types - capture input state alongside rendered output
// ============================================================================

struct ViewTestResult<S: fmt::Debug> {
    state: S,
    rendered: String,
}

impl<S: fmt::Debug> fmt::Display for ViewTestResult<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write state with Debug formatting
        writeln!(f, "State:")?;
        writeln!(f, "{:#?}", self.state)?;
        writeln!(f)?;
        writeln!(f, "Rendered:")?;
        // Write rendered output as-is (already has line-by-line format from TestBackend)
        write!(f, "{}", self.rendered)
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
                .artifacts
                .iter()
                .map(|a| ArtifactSnapshot {
                    target: a.target.clone(),
                    target_type: match a.target_type {
                        TargetType::Nixos => "nixos",
                        TargetType::HomeManager => "homemanager",
                    },
                    name: a.artifact.name.clone(),
                    status: format!("{:?}", a.status),
                    has_logs: !a.step_logs.check.is_empty()
                        || !a.step_logs.generate.is_empty()
                        || !a.step_logs.serialize.is_empty(),
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
    selected_index: usize,
    generators: Vec<GeneratorSnapshot>,
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
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_test_artifact("ssh-key", vec!["passphrase"]),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };
    let entry2 = ArtifactEntry {
        target: "machine-two".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_test_artifact("api-token", vec![]),
        status: ArtifactStatus::UpToDate,
        step_logs: StepLogs::default(),
    };
    let entry3 = ArtifactEntry {
        target: "user@host".to_string(),
        target_type: TargetType::HomeManager,
        artifact: make_test_artifact("gpg-key", vec!["email", "name"]),
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
    };

    Model {
        screen: Screen::ArtifactList,
        artifacts: vec![entry1.clone(), entry2.clone(), entry3.clone()],
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

// Helper to sync changes from artifacts to entries (for tests that modify artifacts directly)
fn sync_artifacts_to_entries(model: &mut Model) {
    for (i, artifact) in model.artifacts.iter().enumerate() {
        if let Some(ListEntry::Single(entry)) = model.entries.get_mut(i) {
            entry.status = artifact.status.clone();
            entry.step_logs = artifact.step_logs.clone();
        }
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
    model.artifacts[1].step_logs.check = vec![LogEntry {
        level: LogLevel::Success,
        message: "Already up to date".to_string(),
    }];
    model.artifacts[1].step_logs.generate = vec![
        LogEntry {
            level: LogLevel::Output,
            message: "Generating API token...".to_string(),
        },
        LogEntry {
            level: LogLevel::Output,
            message: "Token generated successfully".to_string(),
        },
        LogEntry {
            level: LogLevel::Success,
            message: "Generated 1 file(s)".to_string(),
        },
    ];
    model.artifacts[1].step_logs.serialize = vec![LogEntry {
        level: LogLevel::Success,
        message: "Serialized to backend".to_string(),
    }];

    // Sync changes to entries (used for rendering)
    sync_artifacts_to_entries(&mut model);

    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: ArtifactListState::from_model(&model),
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_artifact_list_with_failed_status() {
    let mut model = make_test_model();

    // Update artifacts (legacy field)
    model.artifacts[0].status = ArtifactStatus::Failed {
        error: "Generator script exited with code 1".to_string(),
        output: String::new(),
        retry_available: true,
    };
    model.selected_log_step = LogStep::Generate;
    model.artifacts[0].step_logs.check = vec![LogEntry {
        level: LogLevel::Info,
        message: "Artifact needs regeneration".to_string(),
    }];
    model.artifacts[0].step_logs.generate = vec![
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

    // Update entries (current field used for rendering)
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

    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: ArtifactListState::from_model(&model),
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
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_prompt_with_input() {
    let state = PromptState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
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
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_prompt_hidden_mode() {
    let state = PromptState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
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
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_prompt_multiline_mode() {
    let state = PromptState {
        artifact_index: 0,
        artifact_name: "certificate".to_string(),
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
    };

    let backend = TestBackend::new(60, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_progress(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: GeneratingSnapshot::from_state(&state),
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
    };

    let backend = TestBackend::new(60, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_progress(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: GeneratingSnapshot::from_state(&state),
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
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_multiple_machines_artifact("artifact-one"),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };
    let entry2 = ArtifactEntry {
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_multiple_machines_artifact("artifact-two"),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };
    let entry3 = ArtifactEntry {
        target: "machine-two".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_multiple_machines_artifact("artifact-one"),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };
    let entry4 = ArtifactEntry {
        target: "machine-two".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_multiple_machines_artifact("artifact-two"),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };

    let model = Model {
        screen: Screen::ArtifactList,
        artifacts: vec![
            entry1.clone(),
            entry2.clone(),
            entry3.clone(),
            entry4.clone(),
        ],
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
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_multiple_machines_after_generate_all() {
    let entry1 = ArtifactEntry {
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_multiple_machines_artifact("artifact-one"),
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
    };
    let entry2 = ArtifactEntry {
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_multiple_machines_artifact("artifact-two"),
        status: ArtifactStatus::UpToDate,
        step_logs: StepLogs::default(),
    };
    let entry3 = ArtifactEntry {
        target: "machine-two".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_multiple_machines_artifact("artifact-one"),
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
    };
    let entry4 = ArtifactEntry {
        target: "machine-two".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_multiple_machines_artifact("artifact-two"),
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
    };

    let model = Model {
        screen: Screen::ArtifactList,
        artifacts: vec![
            entry1.clone(),
            entry2.clone(),
            entry3.clone(),
            entry4.clone(),
        ],
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
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_artifact_list_with_shared_artifacts() {
    use artifacts::config::make::SharedArtifactInfo;

    let single_entry = ArtifactEntry {
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_test_artifact("local-secret", vec![]),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };

    let shared_entry = SharedEntry {
        info: SharedArtifactInfo {
            artifact_name: "shared-secret".to_string(),
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
        artifacts: vec![single_entry.clone()],
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
        artifacts: vec![],
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
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_shared_artifact_needs_generation_status() {
    let shared_entry = make_shared_entry_with_status(ArtifactStatus::NeedsGeneration);

    let model = Model {
        screen: Screen::ArtifactList,
        artifacts: vec![],
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
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_shared_artifact_up_to_date_status() {
    let shared_entry = make_shared_entry_with_status(ArtifactStatus::UpToDate);

    let model = Model {
        screen: Screen::ArtifactList,
        artifacts: vec![],
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
        artifacts: vec![],
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
        artifacts: vec![],
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
    };

    let backend = TestBackend::new(70, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_generator_selection(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: GeneratorSelectionSnapshot::from_state(&state),
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_generator_selection_multiple_generators() {
    let state = SelectGeneratorState {
        artifact_index: 0,
        artifact_name: "shared-api-key".to_string(),
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
    };

    let backend = TestBackend::new(70, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_generator_selection(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: GeneratorSelectionSnapshot::from_state(&state),
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_generator_selection_second_selected() {
    let state = SelectGeneratorState {
        artifact_index: 0,
        artifact_name: "shared-api-key".to_string(),
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
    };

    let backend = TestBackend::new(70, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_generator_selection(f, &state, f.area()))
        .unwrap();

    let result = ViewTestResult {
        state: GeneratorSelectionSnapshot::from_state(&state),
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}
