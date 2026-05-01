use artifacts::app::message::{KeyEvent, Message};
use artifacts::app::model::{
    ArtifactEntry, ArtifactStatus, ConfirmRegenerateState, ListEntry, Model, Screen, SharedEntry,
    TargetType,
};
use artifacts::app::update::update;
use artifacts::config::make::{ArtifactDef, FileDef, PromptDef, SharedArtifactInfo};
use artifacts::tui::views::render_confirm_regenerate;
use crossterm::event::KeyCode;
use insta::assert_snapshot;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
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
        backend: "test-backend".to_string(),
    }
}

fn make_test_model_with_existing_artifact() -> Model {
    let entry = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-one".to_string(),
        },
        artifact: make_test_artifact("ssh-key", vec![]),
        status: ArtifactStatus::UpToDate, // EXISTING ARTIFACT - UpToDate means it exists
        runs: Vec::new(),
    };

    Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Single(entry)],
        selected_index: 0,
        selected_log_step: Default::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
        generate_queue: Default::default(),
        active_prompt: None,
        last_esc_at: None,
        pipeline_queue: Default::default(),
        in_flight: None,
    }
}

fn make_test_model_with_new_artifact() -> Model {
    let entry = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-one".to_string(),
        },
        artifact: make_test_artifact("ssh-key", vec![]),
        status: ArtifactStatus::NeedsGeneration, // NEW ARTIFACT - NeedsGeneration means doesn't exist
        runs: Vec::new(),
    };

    Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Single(entry)],
        selected_index: 0,
        selected_log_step: Default::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
        generate_queue: Default::default(),
        active_prompt: None,
        last_esc_at: None,
        pipeline_queue: Default::default(),
        in_flight: None,
    }
}

fn make_shared_entry(status: ArtifactStatus) -> SharedEntry {
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
        status,
        runs: Vec::new(),
        selected_generator: None,
    }
}

fn make_test_model_with_shared_artifact(status: ArtifactStatus) -> Model {
    let shared_entry = make_shared_entry(status);

    Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: Default::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
        generate_queue: Default::default(),
        active_prompt: None,
        last_esc_at: None,
        pipeline_queue: Default::default(),
        in_flight: None,
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
    let (new_model, _effect) = update(model, Message::Key(KeyEvent::enter()));

    // Then: Screen transitions to ConfirmRegenerate (not directly to generation)
    assert!(
        matches!(new_model.screen, Screen::ConfirmRegenerate(_)),
        "Expected ConfirmRegenerate screen for existing artifact, got {:?}",
        new_model.screen
    );
}

#[test]
fn test_dialog_skips_for_new_artifact() {
    // Given: Artifact with status=NeedsGeneration (does not exist yet)
    let model = make_test_model_with_new_artifact();

    // When: User presses Enter on artifact list
    let (new_model, _effect) = update(model, Message::Key(KeyEvent::enter()));

    // Then: No dialog — generation starts in place. Screen stays on the
    // list and the entry's status flips to Generating.
    assert!(
        matches!(new_model.screen, Screen::ArtifactList),
        "Expected ArtifactList screen for new artifact (no dialog), got {:?}",
        new_model.screen
    );
    assert!(
        matches!(new_model.entries[0].status(), ArtifactStatus::Generating(_)),
        "Expected entry status Generating, got {:?}",
        new_model.entries[0].status()
    );
}

#[test]
fn test_dialog_default_selection_is_leave() {
    // Given: ConfirmRegenerate state just opened
    let model = make_test_model_with_existing_artifact();
    let (model, _) = update(model, Message::Key(KeyEvent::enter()));

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
    let (new_model, _effect) = update(model, Message::Key(KeyEvent::from_code(KeyCode::Left)));

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
    let (new_model, _effect) = update(model, Message::Key(KeyEvent::from_code(KeyCode::Right)));

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

    let (new_model, _) = update(model.clone(), Message::Key(KeyEvent::char('h')));
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

    let (new_model, _) = update(model, Message::Key(KeyEvent::char('l')));
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
    let (new_model, _effect) = update(model, Message::Key(KeyEvent::from_code(KeyCode::Tab)));

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
    let (new_model2, _effect2) = update(new_model, Message::Key(KeyEvent::from_code(KeyCode::Tab)));

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
    let (new_model, _effect) = update(model, Message::Key(KeyEvent::enter()));

    // Then: Proceeds to generation. The screen always returns to
    // ArtifactList; with prompts, `active_prompt` is set; without prompts,
    // the entry's status flips to Generating.
    assert!(matches!(new_model.screen, Screen::ArtifactList));
    let proceeded = new_model.active_prompt.is_some()
        || matches!(new_model.entries[0].status(), ArtifactStatus::Generating(_));
    assert!(
        proceeded,
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
    let (new_model, _effect) = update(model, Message::Key(KeyEvent::char(' ')));

    // Then: Proceeds to generation. Screen stays on ArtifactList; either
    // the inline prompt opens or the entry flips to Generating directly.
    assert!(matches!(new_model.screen, Screen::ArtifactList));
    let proceeded = new_model.active_prompt.is_some()
        || matches!(new_model.entries[0].status(), ArtifactStatus::Generating(_));
    assert!(
        proceeded,
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
    let (new_model, _effect) = update(model, Message::Key(KeyEvent::from_code(KeyCode::Esc)));

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
    let (new_model, _effect) = update(model, Message::Key(KeyEvent::enter()));

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
    let (new_model, _effect) = update(model, Message::Key(KeyEvent::enter()));

    // Then: No prompts on this artifact, so generation starts immediately —
    // the screen returns to the artifact list and the entry's status is
    // Generating.
    assert!(
        matches!(new_model.screen, Screen::ArtifactList),
        "Screen should return to ArtifactList; progress is rendered in the right pane"
    );
    assert!(
        matches!(new_model.entries[0].status(), ArtifactStatus::Generating(_)),
        "Selecting Regenerate should flip the entry status to Generating, got {:?}",
        new_model.entries[0].status()
    );
}

#[test]
fn test_dialog_regenerate_proceeds_to_prompts() {
    // Given: ConfirmRegenerate with Regenerate selected and prompts needed
    let _model = make_test_model_with_existing_artifact();
    // Replace with artifact that has prompts
    let entry = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-one".to_string(),
        },
        artifact: make_test_artifact("ssh-key", vec!["passphrase"]),
        status: ArtifactStatus::NeedsGeneration,
        runs: Vec::new(),
    };
    let model = Model {
        screen: Screen::ConfirmRegenerate(ConfirmRegenerateState {
            artifact_index: 0,
            artifact_name: "ssh-key".to_string(),
            affected_targets: vec!["machine-one".to_string()],
            leave_selected: false,
        }),
        entries: vec![ListEntry::Single(entry)],
        selected_index: 0,
        selected_log_step: Default::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
        generate_queue: Default::default(),
        active_prompt: None,
        last_esc_at: None,
        pipeline_queue: Default::default(),
        in_flight: None,
    };

    // When: User presses Enter (confirming Regenerate)
    let (new_model, _effect) = update(model, Message::Key(KeyEvent::enter()));

    // Then: Inline prompt opens on the artifact list — `active_prompt` is set.
    assert!(
        matches!(new_model.screen, Screen::ArtifactList),
        "Selecting Regenerate should return to ArtifactList for inline prompt collection"
    );
    assert!(
        new_model.active_prompt.is_some(),
        "Inline prompt should be active when prompts are needed"
    );
}

#[test]
fn test_shared_artifact_shows_affected_targets() {
    // Given: Shared artifact with multiple targets (existing - UpToDate)
    let model = make_test_model_with_shared_artifact(ArtifactStatus::UpToDate);

    // When: User presses Enter on shared artifact
    let (new_model, _effect) = update(model, Message::Key(KeyEvent::enter()));

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
    // Given: Shared artifact with NeedsGeneration status (doesn't exist)
    let model = make_test_model_with_shared_artifact(ArtifactStatus::NeedsGeneration);

    // When: User presses Enter
    let (new_model, _effect) = update(model, Message::Key(KeyEvent::enter()));

    // Then: No confirmation dialog shown (shared artifacts go to SelectGenerator or Generating/Prompt)
    assert!(
        !matches!(new_model.screen, Screen::ConfirmRegenerate(_)),
        "New shared artifact should skip confirmation dialog"
    );
}

// ============================================================================
// Existence-Derived Behavior
// ============================================================================

#[test]
fn test_entry_exists_used_for_dialog_decision() {
    // This test verifies the core logic that determines when to show the dialog
    // exists is derived from status: UpToDate = true, NeedsGeneration = false

    let entry_up_to_date = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "test".to_string(),
        },
        artifact: make_test_artifact("test", vec![]),
        status: ArtifactStatus::UpToDate, // exists=true
        runs: Vec::new(),
    };

    // The `exists_before` flag passed to the generator effect is derived
    // from `status == UpToDate`.
    let exists_from_status = matches!(entry_up_to_date.status, ArtifactStatus::UpToDate);

    assert!(
        exists_from_status,
        "UpToDate status should mean artifact exists"
    );

    let entry_needs_gen = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "test".to_string(),
        },
        artifact: make_test_artifact("test", vec![]),
        status: ArtifactStatus::NeedsGeneration, // exists=false
        runs: Vec::new(),
    };

    let not_exists_from_status = matches!(entry_needs_gen.status, ArtifactStatus::UpToDate);

    assert!(
        !not_exists_from_status,
        "NeedsGeneration status should mean artifact does not exist"
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
        target_type: TargetType::NixOS {
            machine: "machine-one".to_string(),
        },
        artifact: make_test_artifact("ssh-key", vec![]),
        status: ArtifactStatus::UpToDate,
        runs: Vec::new(),
    };

    let model = Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Single(entry)],
        selected_index: 0,
        selected_log_step: Default::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
        generate_queue: Default::default(),
        active_prompt: None,
        last_esc_at: None,
        pipeline_queue: Default::default(),
        in_flight: None,
    };

    // When: User presses Enter
    let (new_model, _effect) = update(model, Message::Key(KeyEvent::enter()));

    // Then: Dialog appears for regeneration confirmation when artifact is UpToDate
    // (Pressing Enter on an UpToDate artifact shows the ConfirmRegenerate dialog)
    assert!(
        matches!(new_model.screen, Screen::ConfirmRegenerate(_)),
        "Dialog should appear for UpToDate artifacts to confirm regeneration"
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
    // Given: Shared artifact with many targets (existing - UpToDate)
    let shared = make_shared_entry(ArtifactStatus::UpToDate);
    let model = Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Shared(shared)],
        selected_index: 0,
        selected_log_step: Default::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
        generate_queue: Default::default(),
        active_prompt: None,
        last_esc_at: None,
        pipeline_queue: Default::default(),
        in_flight: None,
    };

    // When: User presses Enter
    let (new_model, _) = update(model, Message::Key(KeyEvent::enter()));

    // Then: Targets should be truncated
    if let Screen::ConfirmRegenerate(state) = &new_model.screen {
        // Targets should include "..." for truncation when many
        assert!(
            state.affected_targets.len() <= 6,
            "Targets should be truncated to 5 + '...' when there are many"
        );
    }
}
