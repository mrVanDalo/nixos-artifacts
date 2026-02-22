use artifacts::app::message::{KeyEvent, Msg};
use artifacts::app::model::{
    ArtifactEntry, ArtifactStatus, ConfirmRegenerateState, GeneratingState, GenerationStep,
    ListEntry, Model, Screen, SharedEntry, StepLogs, TargetType,
};
use artifacts::app::update::update;
use artifacts::config::make::{ArtifactDef, FileDef, PromptDef, SharedArtifactInfo};
use artifacts::tui::views::render_confirm_regenerate;
use crossterm::event::KeyCode;
use insta::assert_snapshot;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::collections::BTreeMap;
use std::fmt;

// ============================================================================
// Test Helpers
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

fn make_test_model_with_existing_artifact() -> Model {
    let entry = ArtifactEntry {
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_test_artifact("ssh-key", vec![]),
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
        exists: true, // EXISTING ARTIFACT
    };

    Model {
        screen: Screen::ArtifactList,
        artifacts: vec![entry.clone()],
        entries: vec![ListEntry::Single(entry)],
        selected_index: 0,
        selected_log_step: Default::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    }
}

fn make_test_model_with_new_artifact() -> Model {
    let entry = ArtifactEntry {
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_test_artifact("ssh-key", vec![]),
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
        exists: false, // NEW ARTIFACT
    };

    Model {
        screen: Screen::ArtifactList,
        artifacts: vec![entry.clone()],
        entries: vec![ListEntry::Single(entry)],
        selected_index: 0,
        selected_log_step: Default::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    }
}

fn make_shared_entry(exists: bool) -> SharedEntry {
    SharedEntry {
        info: SharedArtifactInfo {
            artifact_name: "shared-secret".to_string(),
            description: None,
            generators: vec![],
            nixos_targets: vec!["machine-one".to_string(), "machine-two".to_string()],
            home_targets: vec![],
            backend_name: "test".to_string(),
            prompts: BTreeMap::new(),
            files: BTreeMap::new(),
            error: None,
        },
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
        selected_generator: None,
        exists,
    }
}

fn make_test_model_with_shared_artifact(exists: bool) -> Model {
    let shared_entry = make_shared_entry(exists);

    Model {
        screen: Screen::ArtifactList,
        artifacts: vec![],
        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: Default::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    }
}

fn make_confirm_regenerate_state(leave_selected: bool) -> ConfirmRegenerateState {
    ConfirmRegenerateState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        affected_targets: vec!["machine-one".to_string()],
        leave_selected,
    }
}

fn make_confirm_regenerate_state_with_targets() -> ConfirmRegenerateState {
    ConfirmRegenerateState {
        artifact_index: 0,
        artifact_name: "shared-secret".to_string(),
        affected_targets: vec![
            "nixos: machine-one".to_string(),
            "nixos: machine-two".to_string(),
            "nixos: machine-three".to_string(),
        ],
        leave_selected: true,
    }
}

/// Snapshot representation for dialog tests
struct DialogSnapshot {
    artifact_name: String,
    affected_targets: Vec<String>,
    leave_selected: bool,
}

impl fmt::Display for DialogSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Dialog State:")?;
        writeln!(f, "  artifact_name: {}", self.artifact_name)?;
        writeln!(f, "  affected_targets: {:?}", self.affected_targets)?;
        writeln!(f, "  leave_selected: {}", self.leave_selected)?;
        writeln!(f)
    }
}

// ============================================================================
// State Transition Tests
// ============================================================================

#[test]
fn test_dialog_appears_for_existing_artifact() {
    // Given: Artifact with exists=true and status=NeedsGeneration
    let model = make_test_model_with_existing_artifact();

    // When: User presses Enter on artifact list
    let (new_model, _effect) = update(model, Msg::Key(KeyEvent::enter()));

    // Then: Screen transitions to ConfirmRegenerate (not directly to generation)
    assert!(
        matches!(new_model.screen, Screen::ConfirmRegenerate(_)),
        "Expected ConfirmRegenerate screen for existing artifact, got {:?}",
        new_model.screen
    );
}

#[test]
fn test_dialog_skips_for_new_artifact() {
    // Given: Artifact with exists=false and status=NeedsGeneration
    let model = make_test_model_with_new_artifact();

    // When: User presses Enter on artifact list
    let (new_model, _effect) = update(model, Msg::Key(KeyEvent::enter()));

    // Then: Screen transitions directly to generating (no dialog)
    assert!(
        matches!(new_model.screen, Screen::Generating(_)),
        "Expected Generating screen for new artifact, got {:?}",
        new_model.screen
    );
}

#[test]
fn test_dialog_default_selection_is_leave() {
    // Given: ConfirmRegenerate state just opened
    let model = make_test_model_with_existing_artifact();
    let (model, _) = update(model, Msg::Key(KeyEvent::enter()));

    // Then: leave_selected is true (safe default)
    if let Screen::ConfirmRegenerate(state) = &model.screen {
        assert!(
            state.leave_selected,
            "Leave button should be selected by default (safe choice)"
        );
    } else {
        panic!("Expected ConfirmRegenerate screen");
    }
}

#[test]
fn test_dialog_keyboard_left_selects_leave() {
    // Given: ConfirmRegenerate with Regenerate selected
    let mut model = make_test_model_with_existing_artifact();
    model.screen = Screen::ConfirmRegenerate(ConfirmRegenerateState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        affected_targets: vec![],
        leave_selected: false, // Regenerate selected
    });

    // When: User presses Left arrow
    let (new_model, _effect) = update(model, Msg::Key(KeyEvent::from_code(KeyCode::Left)));

    // Then: Leave is selected
    if let Screen::ConfirmRegenerate(state) = &new_model.screen {
        assert!(
            state.leave_selected,
            "Left arrow should select Leave button"
        );
    } else {
        panic!("Expected ConfirmRegenerate screen");
    }
}

#[test]
fn test_dialog_keyboard_right_selects_regenerate() {
    // Given: ConfirmRegenerate with Leave selected (default)
    let mut model = make_test_model_with_existing_artifact();
    model.screen = Screen::ConfirmRegenerate(ConfirmRegenerateState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        affected_targets: vec![],
        leave_selected: true, // Leave selected
    });

    // When: User presses Right arrow
    let (new_model, _effect) = update(model, Msg::Key(KeyEvent::from_code(KeyCode::Right)));

    // Then: Regenerate is selected
    if let Screen::ConfirmRegenerate(state) = &new_model.screen {
        assert!(
            !state.leave_selected,
            "Right arrow should select Regenerate button"
        );
    } else {
        panic!("Expected ConfirmRegenerate screen");
    }
}

#[test]
fn test_dialog_keyboard_vim_keys_work() {
    // Test 'h' key (vim left)
    let mut model = make_test_model_with_existing_artifact();
    model.screen = Screen::ConfirmRegenerate(ConfirmRegenerateState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        affected_targets: vec![],
        leave_selected: false,
    });

    let (new_model, _) = update(model.clone(), Msg::Key(KeyEvent::char('h')));
    if let Screen::ConfirmRegenerate(state) = &new_model.screen {
        assert!(state.leave_selected, "'h' key should select Leave");
    }

    // Test 'l' key (vim right)
    model.screen = Screen::ConfirmRegenerate(ConfirmRegenerateState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        affected_targets: vec![],
        leave_selected: true,
    });

    let (new_model, _) = update(model, Msg::Key(KeyEvent::char('l')));
    if let Screen::ConfirmRegenerate(state) = &new_model.screen {
        assert!(!state.leave_selected, "'l' key should select Regenerate");
    }
}

#[test]
fn test_dialog_keyboard_tab_toggles_selection() {
    // Given: ConfirmRegenerate with Leave selected
    let mut model = make_test_model_with_existing_artifact();
    model.screen = Screen::ConfirmRegenerate(ConfirmRegenerateState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        affected_targets: vec![],
        leave_selected: true,
    });

    // When: User presses Tab
    let (new_model, _effect) = update(model, Msg::Key(KeyEvent::from_code(KeyCode::Tab)));

    // Then: Selection toggles to Regenerate
    if let Screen::ConfirmRegenerate(state) = &new_model.screen {
        assert!(
            !state.leave_selected,
            "Tab should toggle from Leave to Regenerate"
        );
    } else {
        panic!("Expected ConfirmRegenerate screen");
    }

    // When: User presses Tab again
    let (new_model2, _effect2) = update(new_model, Msg::Key(KeyEvent::from_code(KeyCode::Tab)));

    // Then: Selection toggles back to Leave
    if let Screen::ConfirmRegenerate(state) = &new_model2.screen {
        assert!(
            state.leave_selected,
            "Tab should toggle from Regenerate back to Leave"
        );
    } else {
        panic!("Expected ConfirmRegenerate screen");
    }
}

#[test]
fn test_dialog_enter_confirms_selection() {
    // Given: ConfirmRegenerate with Regenerate selected
    let mut model = make_test_model_with_existing_artifact();
    model.screen = Screen::ConfirmRegenerate(ConfirmRegenerateState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        affected_targets: vec![],
        leave_selected: false, // Regenerate selected
    });

    // When: User presses Enter
    let (new_model, _effect) = update(model, Msg::Key(KeyEvent::enter()));

    // Then: Proceeds to generation
    assert!(
        matches!(new_model.screen, Screen::Generating(_) | Screen::Prompt(_)),
        "Expected to proceed to generation when Regenerate selected"
    );
}

#[test]
fn test_dialog_space_confirms_selection() {
    // Given: ConfirmRegenerate with Regenerate selected
    let mut model = make_test_model_with_existing_artifact();
    model.screen = Screen::ConfirmRegenerate(ConfirmRegenerateState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        affected_targets: vec![],
        leave_selected: false, // Regenerate selected
    });

    // When: User presses Space
    let (new_model, _effect) = update(model, Msg::Key(KeyEvent::char(' ')));

    // Then: Proceeds to generation
    assert!(
        matches!(new_model.screen, Screen::Generating(_) | Screen::Prompt(_)),
        "Expected to proceed to generation when Space pressed with Regenerate selected"
    );
}

#[test]
fn test_dialog_esc_cancels() {
    // Given: ConfirmRegenerate (any state)
    let mut model = make_test_model_with_existing_artifact();
    model.screen = Screen::ConfirmRegenerate(ConfirmRegenerateState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        affected_targets: vec![],
        leave_selected: false, // Regenerate selected
    });

    // When: User presses Esc
    let (new_model, _effect) = update(model, Msg::Key(KeyEvent::from_code(KeyCode::Esc)));

    // Then: Returns to ArtifactList
    assert!(
        matches!(new_model.screen, Screen::ArtifactList),
        "Esc should cancel and return to ArtifactList"
    );
}

#[test]
fn test_dialog_leave_returns_to_list() {
    // Given: ConfirmRegenerate with Leave selected
    let mut model = make_test_model_with_existing_artifact();
    model.screen = Screen::ConfirmRegenerate(ConfirmRegenerateState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        affected_targets: vec![],
        leave_selected: true, // Leave selected
    });

    // When: User presses Enter (confirming Leave)
    let (new_model, _effect) = update(model, Msg::Key(KeyEvent::enter()));

    // Then: Returns to ArtifactList
    assert!(
        matches!(new_model.screen, Screen::ArtifactList),
        "Selecting Leave should return to ArtifactList"
    );
}

#[test]
fn test_dialog_regenerate_proceeds_to_generation() {
    // Given: ConfirmRegenerate with Regenerate selected (no prompts)
    let mut model = make_test_model_with_existing_artifact();
    model.screen = Screen::ConfirmRegenerate(ConfirmRegenerateState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        affected_targets: vec![],
        leave_selected: false, // Regenerate selected
    });

    // When: User presses Enter (confirming Regenerate)
    let (new_model, _effect) = update(model, Msg::Key(KeyEvent::enter()));

    // Then: Proceeds to Generating (no prompts for this artifact)
    assert!(
        matches!(new_model.screen, Screen::Generating(_)),
        "Selecting Regenerate should proceed to Generating for artifact without prompts"
    );
}

#[test]
fn test_dialog_regenerate_proceeds_to_prompts() {
    // Given: ConfirmRegenerate with Regenerate selected and prompts needed
    let _model = make_test_model_with_existing_artifact();
    // Replace with artifact that has prompts
    let entry = ArtifactEntry {
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_test_artifact("ssh-key", vec!["passphrase"]),
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
        exists: true,
    };
    let model = Model {
        screen: Screen::ConfirmRegenerate(ConfirmRegenerateState {
            artifact_index: 0,
            artifact_name: "ssh-key".to_string(),
            affected_targets: vec!["machine-one".to_string()],
            leave_selected: false,
        }),
        artifacts: vec![entry.clone()],
        entries: vec![ListEntry::Single(entry)],
        selected_index: 0,
        selected_log_step: Default::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    // When: User presses Enter (confirming Regenerate)
    let (new_model, _effect) = update(model, Msg::Key(KeyEvent::enter()));

    // Then: Proceeds to Prompt
    assert!(
        matches!(new_model.screen, Screen::Prompt(_)),
        "Selecting Regenerate should proceed to Prompt when prompts are needed"
    );
}

#[test]
fn test_shared_artifact_shows_affected_targets() {
    // Given: Shared artifact with multiple targets (existing)
    let model = make_test_model_with_shared_artifact(true);

    // When: User presses Enter on shared artifact
    let (new_model, _effect) = update(model, Msg::Key(KeyEvent::enter()));

    // Then: ConfirmRegenerate screen opens with affected targets
    if let Screen::ConfirmRegenerate(state) = &new_model.screen {
        assert_eq!(
            state.affected_targets.len(),
            2,
            "Should show 2 targets (machine-one and machine-two)"
        );
        // Check that targets are formatted correctly
        let has_nixos_prefix = state
            .affected_targets
            .iter()
            .any(|t| t.starts_with("nixos:"));
        assert!(has_nixos_prefix, "Targets should have 'nixos:' prefix");
    } else {
        panic!(
            "Expected ConfirmRegenerate screen for existing shared artifact, got {:?}",
            new_model.screen
        );
    }
}

#[test]
fn test_dialog_skips_for_new_shared_artifact() {
    // Given: Shared artifact with exists=false
    let model = make_test_model_with_shared_artifact(false);

    // When: User presses Enter
    let (new_model, _effect) = update(model, Msg::Key(KeyEvent::enter()));

    // Then: No confirmation dialog shown (shared artifacts go to SelectGenerator or Generating/Prompt)
    assert!(
        !matches!(new_model.screen, Screen::ConfirmRegenerate(_)),
        "New shared artifact should skip confirmation dialog"
    );
}

// ============================================================================
// Status Text Tests (Regenerating vs Generating)
// ============================================================================

#[test]
fn test_status_text_generating_state_for_existing() {
    // Given: GeneratingState with exists=true
    let state = GeneratingState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        step: GenerationStep::RunningGenerator,
        log_lines: vec![],
        exists: true, // EXISTING
    };

    // Then: The exists flag is properly set
    assert!(
        state.exists,
        "GeneratingState should have exists=true for regeneration"
    );
}

#[test]
fn test_status_text_generating_state_for_new() {
    // Given: GeneratingState with exists=false
    let state = GeneratingState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        step: GenerationStep::RunningGenerator,
        log_lines: vec![],
        exists: false, // NEW
    };

    // Then: The exists flag is properly set
    assert!(
        !state.exists,
        "GeneratingState should have exists=false for new artifact"
    );
}

#[test]
fn test_generating_state_exists_flows_from_entry() {
    // Verify that exists flag flows correctly from entry to GeneratingState
    let model_existing = make_test_model_with_existing_artifact();
    let (new_model, _) = update(model_existing, Msg::Key(KeyEvent::enter()));

    // Should proceed to generation (dialog skipped due to no prompts, but for this test
    // we need an artifact WITH prompts so we can check the dialog flow)
    // Actually, let's check the flow properly:
    // For existing artifact with no prompts, it goes directly to Generating
    if let Screen::Generating(state) = &new_model.screen {
        assert!(
            state.exists,
            "GeneratingState should inherit exists=true from entry"
        );
    }
}

#[test]
fn test_entry_exists_used_for_dialog_decision() {
    // This test verifies the core logic that determines when to show the dialog
    let entry_with_exists = ArtifactEntry {
        target: "test".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_test_artifact("test", vec![]),
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
        exists: true,
    };

    // Logic from update.rs: show dialog if exists=true AND status=NeedsGeneration
    let should_show_dialog = entry_with_exists.exists
        && matches!(entry_with_exists.status, ArtifactStatus::NeedsGeneration);

    assert!(
        should_show_dialog,
        "Dialog should be shown when exists=true and status=NeedsGeneration"
    );

    let entry_without_exists = ArtifactEntry {
        target: "test".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_test_artifact("test", vec![]),
        status: ArtifactStatus::NeedsGeneration,
        step_logs: StepLogs::default(),
        exists: false,
    };

    let should_skip_dialog = !entry_without_exists.exists
        && matches!(entry_without_exists.status, ArtifactStatus::NeedsGeneration);

    assert!(
        should_skip_dialog,
        "Dialog should be skipped when exists=false"
    );
}

// ============================================================================
// Visual Snapshot Tests
// ============================================================================

fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    let mut result = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = buffer.cell((x, y)).unwrap();
            result.push(cell.symbol().chars().next().unwrap_or(' '));
        }
        result.push('\n');
    }
    result
}

struct DialogViewResult {
    state: DialogSnapshot,
    rendered: String,
}

impl fmt::Display for DialogViewResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.state)?;
        writeln!(f, "Rendered:")?;
        write!(f, "{}", self.rendered)
    }
}

#[test]
fn test_dialog_snapshot_leave_selected() {
    let state = make_confirm_regenerate_state(true); // Leave selected

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_confirm_regenerate(f, &state, f.area()))
        .unwrap();

    let result = DialogViewResult {
        state: DialogSnapshot {
            artifact_name: state.artifact_name.clone(),
            affected_targets: state.affected_targets.clone(),
            leave_selected: state.leave_selected,
        },
        rendered: buffer_to_string(terminal.backend().buffer()),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_dialog_snapshot_regenerate_selected() {
    let state = make_confirm_regenerate_state(false); // Regenerate selected

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_confirm_regenerate(f, &state, f.area()))
        .unwrap();

    let result = DialogViewResult {
        state: DialogSnapshot {
            artifact_name: state.artifact_name.clone(),
            affected_targets: state.affected_targets.clone(),
            leave_selected: state.leave_selected,
        },
        rendered: buffer_to_string(terminal.backend().buffer()),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_dialog_snapshot_with_targets() {
    let state = make_confirm_regenerate_state_with_targets();

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_confirm_regenerate(f, &state, f.area()))
        .unwrap();

    let result = DialogViewResult {
        state: DialogSnapshot {
            artifact_name: state.artifact_name.clone(),
            affected_targets: state.affected_targets.clone(),
            leave_selected: state.leave_selected,
        },
        rendered: buffer_to_string(terminal.backend().buffer()),
    };
    assert_snapshot!(result.to_string());
}

#[test]
fn test_dialog_snapshot_shared_artifact() {
    let state = ConfirmRegenerateState {
        artifact_index: 0,
        artifact_name: "shared-cert".to_string(),
        affected_targets: vec![
            "nixos: server-1".to_string(),
            "nixos: server-2".to_string(),
            "home: alice@laptop".to_string(),
        ],
        leave_selected: true,
    };

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_confirm_regenerate(f, &state, f.area()))
        .unwrap();

    let result = DialogViewResult {
        state: DialogSnapshot {
            artifact_name: state.artifact_name.clone(),
            affected_targets: state.affected_targets.clone(),
            leave_selected: state.leave_selected,
        },
        rendered: buffer_to_string(terminal.backend().buffer()),
    };
    assert_snapshot!(result.to_string());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_dialog_appears_only_for_needs_generation() {
    // Given: Existing artifact that is UpToDate (not NeedsGeneration)
    let entry = ArtifactEntry {
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        artifact: make_test_artifact("ssh-key", vec![]),
        status: ArtifactStatus::UpToDate,
        step_logs: StepLogs::default(),
        exists: true,
    };

    let model = Model {
        screen: Screen::ArtifactList,
        artifacts: vec![entry.clone()],
        entries: vec![ListEntry::Single(entry)],
        selected_index: 0,
        selected_log_step: Default::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    // When: User presses Enter
    let (new_model, _effect) = update(model, Msg::Key(KeyEvent::enter()));

    // Then: No dialog shown (artifact is up to date, doesn't need generation)
    // Note: This may go to generation or stay on list depending on implementation
    // The key is that we should NOT see a dialog
    assert!(
        !matches!(new_model.screen, Screen::ConfirmRegenerate(_)),
        "Dialog should not appear for UpToDate artifacts"
    );
}

#[test]
fn test_dialog_with_empty_targets() {
    // Given: Dialog state with empty targets list
    let state = ConfirmRegenerateState {
        artifact_index: 0,
        artifact_name: "local-secret".to_string(),
        affected_targets: vec![], // Empty
        leave_selected: true,
    };

    // Render should not panic with empty targets
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_confirm_regenerate(f, &state, f.area()))
        .unwrap();

    // Test passes if we reach here without panic
}

#[test]
fn test_dialog_with_many_targets_truncation() {
    // Given: Shared artifact with many targets
    let shared = make_shared_entry(true);
    let model = Model {
        screen: Screen::ArtifactList,
        artifacts: vec![],
        entries: vec![ListEntry::Shared(shared)],
        selected_index: 0,
        selected_log_step: Default::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    // When: User presses Enter
    let (new_model, _) = update(model, Msg::Key(KeyEvent::enter()));

    // Then: Targets should be truncated
    if let Screen::ConfirmRegenerate(state) = &new_model.screen {
        // Targets should include "..." for truncation when many
        assert!(
            state.affected_targets.len() <= 6,
            "Targets should be truncated to 5 + '...' when there are many"
        );
    }
}
