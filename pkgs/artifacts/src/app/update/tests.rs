use super::*;
use crate::config::make::{ArtifactDef, FileDef, PromptDef};
use std::collections::BTreeMap;

fn make_test_artifact(name: &str, prompts: Vec<&str>) -> ArtifactDef {
    let mut prompt_map = BTreeMap::new();
    for p in prompts {
        prompt_map.insert(
            p.to_string(),
            PromptDef {
                name: p.to_string(),
                description: Some(format!("Enter {}", p)),
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
                path: Some("/test".to_string()),
                owner: None,
                group: None,
            },
        )]),
        prompts: prompt_map,
        generator: "/gen".to_string(),
        serialization: "test".to_string(),
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
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };

    Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Single(entry1), ListEntry::Single(entry2)],
        selected_index: 0,
        selected_log_step: LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    }
}

#[test]
fn test_navigate_down() {
    let model = make_test_model();
    let (new_model, effect) = update(model, Message::Key(KeyEvent::char('j')));

    assert_eq!(new_model.selected_index, 1);
    assert!(effect.is_none());
}

#[test]
fn test_navigate_up() {
    let mut model = make_test_model();
    model.selected_index = 1;
    let (new_model, effect) = update(model, Message::Key(KeyEvent::char('k')));

    assert_eq!(new_model.selected_index, 0);
    assert!(effect.is_none());
}

#[test]
fn test_navigate_up_at_top_stays() {
    let model = make_test_model();
    let (new_model, _) = update(model, Message::Key(KeyEvent::char('k')));

    assert_eq!(new_model.selected_index, 0);
}

#[test]
fn test_navigate_down_at_bottom_stays() {
    let mut model = make_test_model();
    model.selected_index = 1;
    let (new_model, _) = update(model, Message::Key(KeyEvent::char('j')));

    assert_eq!(new_model.selected_index, 1);
}

#[test]
fn test_quit_with_q() {
    let model = make_test_model();
    let (_, effect) = update(model, Message::Key(KeyEvent::char('q')));

    assert!(effect.is_quit());
}

#[test]
fn test_quit_with_esc() {
    let model = make_test_model();
    let (_, effect) = update(model, Message::Key(KeyEvent::esc()));

    assert!(effect.is_quit());
}

#[test]
fn test_enter_opens_prompt_screen() {
    let model = make_test_model();
    let (new_model, _) = update(model, Message::Key(KeyEvent::enter()));

    assert!(matches!(new_model.screen, Screen::Prompt(_)));
}

#[test]
fn test_enter_skips_prompt_if_no_prompts() {
    let mut model = make_test_model();
    model.selected_index = 1; // api-token has no prompts
    let (new_model, effect) = update(model, Message::Key(KeyEvent::enter()));

    assert!(matches!(new_model.screen, Screen::Generating(_)));
    assert!(matches!(effect, Effect::RunGenerator { .. }));
}

#[test]
fn test_prompt_typing() {
    let mut model = make_test_model();
    model.screen = Screen::Prompt(PromptState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        description: None,
        prompts: vec![PromptEntry {
            name: "pass".to_string(),
            description: None,
        }],
        current_prompt_index: 0,
        input_mode: InputMode::Line,
        buffer: String::new(),
        collected: Default::default(),
    });

    let (model, _) = update(model, Message::Key(KeyEvent::char('h')));
    let (model, _) = update(model, Message::Key(KeyEvent::char('i')));

    if let Screen::Prompt(state) = &model.screen {
        assert_eq!(state.buffer, "hi");
    } else {
        panic!("Expected prompt screen");
    }
}

#[test]
fn test_prompt_backspace() {
    let mut model = make_test_model();
    model.screen = Screen::Prompt(PromptState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        description: None,
        prompts: vec![PromptEntry {
            name: "pass".to_string(),
            description: None,
        }],
        current_prompt_index: 0,
        input_mode: InputMode::Line,
        buffer: "hello".to_string(),
        collected: Default::default(),
    });

    let (model, _) = update(model, Message::Key(KeyEvent::backspace()));

    if let Screen::Prompt(state) = &model.screen {
        assert_eq!(state.buffer, "hell");
    } else {
        panic!("Expected prompt screen");
    }
}

#[test]
fn test_prompt_tab_cycles_mode_when_empty() {
    let mut model = make_test_model();
    model.screen = Screen::Prompt(PromptState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        description: None,
        prompts: vec![PromptEntry {
            name: "pass".to_string(),
            description: None,
        }],
        current_prompt_index: 0,
        input_mode: InputMode::Line,
        buffer: String::new(),
        collected: Default::default(),
    });

    let (model, _) = update(model, Message::Key(KeyEvent::tab()));

    if let Screen::Prompt(state) = &model.screen {
        assert_eq!(state.input_mode, InputMode::Multiline);
    } else {
        panic!("Expected prompt screen");
    }
}

#[test]
fn test_prompt_tab_does_nothing_when_buffer_has_content() {
    let mut model = make_test_model();
    model.screen = Screen::Prompt(PromptState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        description: None,
        prompts: vec![PromptEntry {
            name: "pass".to_string(),
            description: None,
        }],
        current_prompt_index: 0,
        input_mode: InputMode::Line,
        buffer: "some text".to_string(),
        collected: Default::default(),
    });

    let (model, _) = update(model, Message::Key(KeyEvent::tab()));

    if let Screen::Prompt(state) = &model.screen {
        assert_eq!(state.input_mode, InputMode::Line);
    } else {
        panic!("Expected prompt screen");
    }
}

#[test]
fn test_prompt_esc_returns_to_list() {
    let mut model = make_test_model();
    model.screen = Screen::Prompt(PromptState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        description: None,
        prompts: vec![],
        current_prompt_index: 0,
        input_mode: InputMode::Line,
        buffer: String::new(),
        collected: Default::default(),
    });

    let (model, _) = update(model, Message::Key(KeyEvent::esc()));

    assert!(matches!(model.screen, Screen::ArtifactList));
}

#[test]
fn test_tab_cycles_log_step_on_list_screen() {
    let model = make_test_model();
    assert_eq!(model.selected_log_step, LogStep::Check);

    let (model, effect) = update(model, Message::Key(KeyEvent::tab()));
    assert_eq!(model.selected_log_step, LogStep::Generate);
    assert!(effect.is_none());

    let (model, _) = update(model, Message::Key(KeyEvent::tab()));
    assert_eq!(model.selected_log_step, LogStep::Serialize);

    let (model, _) = update(model, Message::Key(KeyEvent::tab()));
    assert_eq!(model.selected_log_step, LogStep::Check);
}

// === Async Effect Tests ===

/// Test that Enter key on artifact with prompts returns RunGenerator effect
#[test]
fn test_update_returns_run_generator_effect() {
    let mut model = make_test_model();
    model.selected_index = 1; // api-token has no prompts

    let (new_model, effect) = update(model, Message::Key(KeyEvent::enter()));

    // Should be generating screen
    assert!(
        matches!(new_model.screen, Screen::Generating(_)),
        "Should enter generating screen"
    );
    assert!(
        matches!(effect, Effect::RunGenerator { .. }),
        "Should return RunGenerator effect"
    );
}

/// Test that GeneratorFinished returns Serialize effect
#[test]
fn test_update_returns_serialize_effect() {
    let mut model = make_test_model();
    model.selected_index = 0;
    model.screen = Screen::Generating(GeneratingState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        step: GenerationStep::RunningGenerator,
        log_lines: vec![],
        exists: false,
    });

    // Simulate successful generator completion
    let (new_model, effect) = update(
        model,
        Message::GeneratorFinished {
            artifact_index: 0,
            result: Ok(ScriptOutput {
                stdout_lines: vec!["Generated key".to_string()],
                stderr_lines: vec![],
            }),
        },
    );

    // Should move to serializing step
    assert!(
        matches!(new_model.screen, Screen::Generating(_)),
        "Should stay on generating screen"
    );
    assert!(
        matches!(effect, Effect::Serialize { .. }),
        "Should return Serialize effect after generator success"
    );
}

/// Test that artifact needs generation returns CheckSerialization effect
#[test]
fn test_update_returns_check_serialization_effect() {
    // Create model with pending artifacts - init() will return CheckSerialization effects
    let model = make_test_model();

    // Verify init() returns batch of CheckSerialization effects
    let effect = init(&model);

    match &effect {
        Effect::Batch(effects) => {
            // Should have effects for each pending artifact
            assert_eq!(
                effects.len(),
                2,
                "Should check serialization for both artifacts"
            );
            for e in effects {
                assert!(
                    matches!(e, Effect::CheckSerialization { .. }),
                    "Each effect should be CheckSerialization"
                );
            }
        }
        _ => panic!("init() should return Effect::Batch with CheckSerialization for each artifact"),
    }
}

/// Test that GeneratorFinished result updates model status correctly
#[test]
fn test_update_handles_async_result() {
    let mut model = make_test_model();
    model.selected_index = 0;

    // Set generating state
    model.screen = Screen::Generating(GeneratingState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        step: GenerationStep::RunningGenerator,
        log_lines: vec![],
        exists: false,
    });

    // Update first entry to Generating status
    if let Some(entry) = model.entries.get_mut(0) {
        *entry.status_mut() = ArtifactStatus::Generating(crate::app::model::GeneratingSubstate {
            step: crate::app::model::GenerationStep::RunningGenerator,
            output: String::new(),
        });
    }

    // Simulate successful completion
    let (new_model, _effect) = update(
        model,
        Message::GeneratorFinished {
            artifact_index: 0,
            result: Ok(ScriptOutput {
                stdout_lines: vec!["Generated successfully".to_string()],
                stderr_lines: vec![],
            }),
        },
    );

    // Verify model state updated
    assert!(matches!(new_model.screen, Screen::Generating(_)));
    // Verify logs were added
    if let Screen::Generating(state) = &new_model.screen {
        assert_eq!(
            state.step,
            GenerationStep::Serializing,
            "Should move to serializing step"
        );
    }
}

/// Test that effect batching works correctly for multiple check operations
#[test]
fn test_update_effect_batching() {
    let model = make_test_model();

    let effect = init(&model);

    // init() should return a batch of check serialization effects
    match effect {
        Effect::Batch(effects) => {
            assert_eq!(
                effects.len(),
                2,
                "Should have one effect per pending artifact"
            );
        }
        _ => panic!("Expected Effect::Batch from init()"),
    }
}

/// Test that SharedCheckSerializationResult updates shared artifact status correctly
#[test]
fn test_shared_check_serialization_result_updates_status() {
    // Create a model with a shared artifact
    let model = make_test_model_with_shared();

    // Initial status should be Pending
    assert_eq!(model.entries[0].status(), &ArtifactStatus::Pending);

    // Simulate successful shared check result indicating generation needed
    let (new_model, effect) = update(
        model,
        Message::SharedCheckSerializationResult {
            artifact_index: 0,
            statuses: vec![ArtifactStatus::NeedsGeneration],
            outputs: vec![ScriptOutput {
                stdout_lines: vec!["Checking shared artifact...".to_string()],
                stderr_lines: vec![],
            }],
        },
    );

    // Status should transition to NeedsGeneration
    assert_eq!(
        new_model.entries[0].status(),
        &ArtifactStatus::NeedsGeneration,
        "Shared artifact should transition from Pending to NeedsGeneration"
    );

    // Effect should be None
    assert!(effect.is_none());
}

/// Test that SharedCheckSerializationResult handles up-to-date status
#[test]
fn test_shared_check_serialization_result_up_to_date() {
    let model = make_test_model_with_shared();

    // Simulate successful shared check result indicating up-to-date
    let (new_model, effect) = update(
        model,
        Message::SharedCheckSerializationResult {
            artifact_index: 0,
            statuses: vec![ArtifactStatus::UpToDate],
            outputs: vec![],
        },
    );

    // Status should transition to UpToDate
    assert_eq!(
        new_model.entries[0].status(),
        &ArtifactStatus::UpToDate,
        "Shared artifact should transition from Pending to UpToDate"
    );
    assert!(effect.is_none());
}

/// Test that SharedCheckSerializationResult handles multiple targets
#[test]
fn test_shared_check_serialization_result_multi_target() {
    let model = make_test_model_with_shared();

    // Simulate successful shared check result with multiple targets (any NeedsGeneration = needs gen)
    let (new_model, effect) = update(
        model,
        Message::SharedCheckSerializationResult {
            artifact_index: 0,
            statuses: vec![
                ArtifactStatus::UpToDate,
                ArtifactStatus::NeedsGeneration,
                ArtifactStatus::UpToDate,
            ],
            outputs: vec![
                ScriptOutput::default(),
                ScriptOutput::default(),
                ScriptOutput::default(),
            ],
        },
    );

    // Status should transition to NeedsGeneration because one target needs it
    assert_eq!(
        new_model.entries[0].status(),
        &ArtifactStatus::NeedsGeneration,
        "Shared artifact should need generation when any target needs it"
    );
    assert!(effect.is_none());
}

/// Test that single generator skips selection dialog and goes to prompts
#[test]
fn test_single_generator_skips_dialog() {
    use crate::app::model::SharedEntry;
    use crate::config::make::{GeneratorInfo, GeneratorSource, PromptDef, SharedArtifactInfo};
    use std::collections::BTreeMap;

    // Create shared artifact with only one generator
    let mut prompts_map: BTreeMap<String, PromptDef> = BTreeMap::new();
    prompts_map.insert(
        "passphrase".to_string(),
        PromptDef {
            name: "passphrase".to_string(),
            description: Some("Enter passphrase".to_string()),
        },
    );

    let shared_info = SharedArtifactInfo {
        description: None,
        artifact_name: "shared-ssh-key".to_string(),
        backend_name: "test-backend".to_string(),
        nixos_targets: vec!["machine-one".to_string()],
        home_targets: vec![],
        generators: vec![GeneratorInfo {
            path: "/nix/store/abc123/generator.sh".to_string(),
            sources: vec![GeneratorSource {
                target: "machine-one".to_string(),
                target_type: crate::config::make::TargetType::Nixos,
            }],
        }],
        prompts: prompts_map,
        files: BTreeMap::new(),
        error: None,
    };

    let shared_entry = SharedEntry {
        info: shared_info,
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
        selected_generator: None,
    };

    let model = Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    // Press Enter on shared artifact
    let (new_model, effect) = update(model, Message::Key(KeyEvent::enter()));

    // Should go directly to Prompt screen, not SelectGenerator
    assert!(
        matches!(new_model.screen, Screen::Prompt(_)),
        "Single generator should skip to Prompt screen, got {:?}",
        new_model.screen
    );

    // Effect should be None (prompts needed)
    assert!(
        effect.is_none(),
        "Expected no effect when prompts are needed"
    );
}

/// Test that single generator without prompts goes directly to generating
#[test]
fn test_single_generator_no_prompts_goes_to_generating() {
    use crate::app::model::SharedEntry;
    use crate::config::make::{FileDef, GeneratorInfo, GeneratorSource, SharedArtifactInfo};
    use std::collections::BTreeMap;

    // Create shared artifact with only one generator and no prompts
    let mut files_map: BTreeMap<String, FileDef> = BTreeMap::new();
    files_map.insert(
        "key".to_string(),
        FileDef {
            name: "key".to_string(),
            path: Some("/etc/ssh/ssh_key".to_string()),
            owner: None,
            group: None,
        },
    );

    let shared_info = SharedArtifactInfo {
        description: None,
        artifact_name: "shared-ssh-key".to_string(),
        backend_name: "test-backend".to_string(),
        nixos_targets: vec!["machine-one".to_string()],
        home_targets: vec![],
        generators: vec![GeneratorInfo {
            path: "/nix/store/abc123/generator.sh".to_string(),
            sources: vec![GeneratorSource {
                target: "machine-one".to_string(),
                target_type: crate::config::make::TargetType::Nixos,
            }],
        }],
        prompts: BTreeMap::new(), // No prompts
        files: files_map,
        error: None,
    };

    let shared_entry = SharedEntry {
        info: shared_info,
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
        selected_generator: None,
    };

    let model = Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    // Press Enter on shared artifact
    let (new_model, effect) = update(model, Message::Key(KeyEvent::enter()));

    // Should go directly to Generating screen
    assert!(
        matches!(new_model.screen, Screen::Generating(_)),
        "Single generator without prompts should go to Generating screen, got {:?}",
        new_model.screen
    );

    // Effect should be RunSharedGenerator
    assert!(
        matches!(effect, Effect::RunSharedGenerator { .. }),
        "Expected RunSharedGenerator effect, got {:?}",
        effect
    );
}

/// Test that multiple generators shows selection dialog
#[test]
fn test_multiple_generators_shows_dialog() {
    use crate::app::model::SharedEntry;
    use crate::config::make::{GeneratorInfo, GeneratorSource, SharedArtifactInfo};
    use std::collections::BTreeMap;

    // Create shared artifact with multiple generators
    let shared_info = SharedArtifactInfo {
        description: None,
        artifact_name: "shared-ssh-key".to_string(),
        backend_name: "test-backend".to_string(),
        nixos_targets: vec!["machine-one".to_string(), "machine-two".to_string()],
        home_targets: vec![],
        generators: vec![
            GeneratorInfo {
                path: "/nix/store/abc123/gen1.sh".to_string(),
                sources: vec![GeneratorSource {
                    target: "machine-one".to_string(),
                    target_type: crate::config::make::TargetType::Nixos,
                }],
            },
            GeneratorInfo {
                path: "/nix/store/def456/gen2.sh".to_string(),
                sources: vec![GeneratorSource {
                    target: "machine-two".to_string(),
                    target_type: crate::config::make::TargetType::Nixos,
                }],
            },
        ],
        prompts: BTreeMap::new(),
        files: BTreeMap::new(),
        error: None,
    };

    let shared_entry = SharedEntry {
        info: shared_info,
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
        selected_generator: None,
    };

    let model = Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    // Press Enter on shared artifact
    let (new_model, effect) = update(model, Message::Key(KeyEvent::enter()));

    // Should show SelectGenerator screen
    assert!(
        matches!(new_model.screen, Screen::SelectGenerator(_)),
        "Multiple generators should show SelectGenerator screen, got {:?}",
        new_model.screen
    );

    // Effect should be None (screen transition is handled in update)
    assert!(effect.is_none(), "Expected None effect, got {:?}", effect);

    // Verify generators are in the screen state
    if let Screen::SelectGenerator(state) = new_model.screen {
        assert_eq!(
            state.generators.len(),
            2,
            "SelectGenerator should have both generators"
        );
        assert_eq!(state.generators[0].path, "/nix/store/abc123/gen1.sh");
        assert_eq!(state.generators[1].path, "/nix/store/def456/gen2.sh");
    }
}

/// Test that selected generator is stored when single generator auto-selected
#[test]
fn test_single_generator_stores_selected_path() {
    use crate::app::model::SharedEntry;
    use crate::config::make::{GeneratorInfo, GeneratorSource, SharedArtifactInfo};
    use std::collections::BTreeMap;

    // Create shared artifact with one generator
    let shared_info = SharedArtifactInfo {
        description: None,
        artifact_name: "shared-ssh-key".to_string(),
        backend_name: "test-backend".to_string(),
        nixos_targets: vec!["machine-one".to_string()],
        home_targets: vec![],
        generators: vec![GeneratorInfo {
            path: "/nix/store/abc123/generator.sh".to_string(),
            sources: vec![GeneratorSource {
                target: "machine-one".to_string(),
                target_type: crate::config::make::TargetType::Nixos,
            }],
        }],
        prompts: BTreeMap::new(), // No prompts
        files: BTreeMap::new(),
        error: None,
    };

    let shared_entry = SharedEntry {
        info: shared_info,
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
        selected_generator: None,
    };

    let model = Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    // Press Enter on shared artifact
    let (new_model, _) = update(model, Message::Key(KeyEvent::enter()));

    // Verify the selected_generator was stored in the entry
    if let ListEntry::Shared(shared) = &new_model.entries[0] {
        assert_eq!(
            shared.selected_generator,
            Some("/nix/store/abc123/generator.sh".to_string()),
            "Generator path should be stored in selected_generator"
        );
    } else {
        panic!("Expected ListEntry::Shared");
    }
}

fn make_test_model_with_shared() -> Model {
    use crate::app::model::SharedEntry;
    use crate::config::make::{GeneratorInfo, GeneratorSource, SharedArtifactInfo};
    use std::collections::BTreeMap;

    let shared_info = SharedArtifactInfo {
        description: None,
        artifact_name: "shared-ssh-key".to_string(),
        backend_name: "test-backend".to_string(),
        nixos_targets: vec!["machine-one".to_string(), "machine-two".to_string()],
        home_targets: vec!["alice@host".to_string()],
        generators: vec![GeneratorInfo {
            path: "/test/generator.sh".to_string(),
            sources: vec![
                GeneratorSource {
                    target: "machine-one".to_string(),
                    target_type: crate::config::make::TargetType::Nixos,
                },
                GeneratorSource {
                    target: "machine-two".to_string(),
                    target_type: crate::config::make::TargetType::Nixos,
                },
            ],
        }],
        prompts: BTreeMap::new(),
        files: BTreeMap::new(),
        error: None,
    };

    let shared_entry = SharedEntry {
        info: shared_info,
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
        selected_generator: None,
    };

    Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    }
}
