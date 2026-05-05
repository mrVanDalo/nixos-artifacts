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
        backend: "test".to_string(),
    }
}

fn make_test_model() -> Model {
    let entry1 = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-one".to_string(),
        },
        artifact: make_test_artifact("ssh-key", vec!["passphrase"]),
        status: ArtifactStatus::Pending,
        runs: Vec::new(),
    };
    let entry2 = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-two".to_string(),
        },
        artifact: make_test_artifact("api-token", vec![]),
        status: ArtifactStatus::Pending,
        runs: Vec::new(),
    };

    Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Single(entry1), ListEntry::Single(entry2)],
        selected_index: 0,
        selected_log_step: Step::default(),
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
fn test_esc_on_artifact_list_does_not_quit() {
    // Esc on the artifact list is reserved for the Esc-Esc cancel chord —
    // single Esc must NOT quit (only `q` quits). The first Esc records the
    // chord timestamp and otherwise leaves the model unchanged.
    let model = make_test_model();
    let (new_model, effect) = update(model, Message::Key(KeyEvent::esc()));

    assert!(!effect.is_quit(), "single Esc on list must not quit");
    assert!(effect.is_none());
    assert!(matches!(new_model.screen, Screen::ArtifactList));
    assert!(
        new_model.last_esc_at.is_some(),
        "first Esc must seed the chord timer"
    );
}

#[test]
fn test_enter_opens_inline_prompt() {
    let model = make_test_model();
    let (new_model, _) = update(model, Message::Key(KeyEvent::enter()));

    // Inline prompt: stays on the artifact list, prompt state lives in
    // `active_prompt`.
    assert!(matches!(new_model.screen, Screen::ArtifactList));
    assert!(
        new_model.active_prompt.is_some(),
        "expected active_prompt to be set after Enter on prompt-bearing artifact"
    );
}

#[test]
fn test_enter_skips_prompt_if_no_prompts() {
    let mut model = make_test_model();
    model.selected_index = 1; // api-token has no prompts
    let (new_model, effect) = update(model, Message::Key(KeyEvent::enter()));

    assert!(matches!(new_model.screen, Screen::ArtifactList));
    assert!(matches!(
        new_model.entries[1].status(),
        ArtifactStatus::Generating(_)
    ));
    assert!(matches!(effect, Effect::RunGenerator { .. }));
}

fn make_active_prompt(buffer: &str, mode: InputMode) -> PromptState {
    PromptState {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        description: None,
        prompts: vec![PromptEntry {
            name: "pass".to_string(),
            description: None,
        }],
        current_prompt_index: 0,
        input_mode: mode,
        buffer: buffer.to_string(),
        collected: Default::default(),
    }
}

#[test]
fn test_prompt_typing() {
    let mut model = make_test_model();
    model.active_prompt = Some(make_active_prompt("", InputMode::Line));

    let (model, _) = update(model, Message::Key(KeyEvent::char('h')));
    let (model, _) = update(model, Message::Key(KeyEvent::char('i')));

    let state = model
        .active_prompt
        .as_ref()
        .expect("active_prompt should still be set after typing");
    assert_eq!(state.buffer, "hi");
}

#[test]
fn test_prompt_backspace() {
    let mut model = make_test_model();
    model.active_prompt = Some(make_active_prompt("hello", InputMode::Line));

    let (model, _) = update(model, Message::Key(KeyEvent::backspace()));

    let state = model.active_prompt.as_ref().unwrap();
    assert_eq!(state.buffer, "hell");
}

#[test]
fn test_prompt_tab_cycles_mode_when_empty() {
    let mut model = make_test_model();
    model.active_prompt = Some(make_active_prompt("", InputMode::Line));

    let (model, _) = update(model, Message::Key(KeyEvent::tab()));

    let state = model.active_prompt.as_ref().unwrap();
    assert_eq!(state.input_mode, InputMode::Multiline);
}

#[test]
fn test_prompt_tab_does_nothing_when_buffer_has_content() {
    let mut model = make_test_model();
    model.active_prompt = Some(make_active_prompt("some text", InputMode::Line));

    let (model, _) = update(model, Message::Key(KeyEvent::tab()));

    let state = model.active_prompt.as_ref().unwrap();
    assert_eq!(state.input_mode, InputMode::Line);
}

#[test]
fn test_prompt_esc_clears_active_prompt() {
    let mut model = make_test_model();
    model.active_prompt = Some(make_active_prompt("", InputMode::Line));

    let (model, _) = update(model, Message::Key(KeyEvent::esc()));

    assert!(matches!(model.screen, Screen::ArtifactList));
    assert!(
        model.active_prompt.is_none(),
        "Esc should clear active_prompt"
    );
}

#[test]
fn test_tab_cycles_log_step_on_list_screen() {
    let model = make_test_model();
    assert_eq!(model.selected_log_step, Step::Check);

    let (model, effect) = update(model, Message::Key(KeyEvent::tab()));
    assert_eq!(model.selected_log_step, Step::Generate);
    assert!(effect.is_none());

    let (model, _) = update(model, Message::Key(KeyEvent::tab()));
    assert_eq!(model.selected_log_step, Step::Serialize);

    let (model, _) = update(model, Message::Key(KeyEvent::tab()));
    assert_eq!(model.selected_log_step, Step::Check);
}

// === Async Effect Tests ===

/// Test that Enter key on artifact with prompts returns RunGenerator effect
#[test]
fn test_update_returns_run_generator_effect() {
    let mut model = make_test_model();
    model.selected_index = 1; // api-token has no prompts

    let (new_model, effect) = update(model, Message::Key(KeyEvent::enter()));

    // Generation no longer takes over the screen — progress renders in the
    // right pane while the user stays on ArtifactList.
    assert!(matches!(new_model.screen, Screen::ArtifactList));
    assert!(
        matches!(new_model.entries[1].status(), ArtifactStatus::Generating(_)),
        "Selected artifact should be Generating"
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
    *model.entries[0].status_mut() = ArtifactStatus::Generating(GeneratingSubstate {
        step: Step::Generate,
        output: String::new(),
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

    // Status stays Generating while serialize runs; the screen never leaves
    // the artifact list.
    assert!(matches!(new_model.screen, Screen::ArtifactList));
    assert!(
        matches!(new_model.entries[0].status(), ArtifactStatus::Generating(_)),
        "Should stay Generating until serialize completes"
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
    let mut model = make_test_model();

    // Verify init() returns batch of CheckSerialization effects
    let effect = init(&mut model);

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

    // Update first entry to Generating status
    if let Some(entry) = model.entries.get_mut(0) {
        *entry.status_mut() = ArtifactStatus::Generating(GeneratingSubstate {
            step: Step::Generate,
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

    // Status still Generating, but the substate has advanced to the
    // Serialize step. Screen never leaves the artifact list.
    assert!(matches!(new_model.screen, Screen::ArtifactList));
    if let ArtifactStatus::Generating(substate) = new_model.entries[0].status() {
        assert_eq!(
            substate.step,
            Step::Serialize,
            "Should move to serializing step"
        );
    } else {
        panic!(
            "expected Generating substate, got {:?}",
            new_model.entries[0].status()
        );
    }
}

/// Test that effect batching works correctly for multiple check operations
#[test]
fn test_update_effect_batching() {
    let mut model = make_test_model();

    let effect = init(&mut model);

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

/// Test that CheckSerializationResult updates shared artifact status correctly
/// Note: With unified messages, aggregation happens in background handler
#[test]
fn test_shared_check_serialization_result_updates_status() {
    // Create a model with a shared artifact
    let model = make_test_model_with_shared();

    // Initial status should be Pending
    assert_eq!(model.entries[0].status(), &ArtifactStatus::Pending);

    // Simulate successful check result indicating generation needed
    // (background handler now aggregates to single status before sending)
    let (new_model, effect) = update(
        model,
        Message::CheckSerializationResult {
            artifact_index: 0,
            status: ArtifactStatus::NeedsGeneration,
            result: Ok(ScriptOutput {
                stdout_lines: vec!["Checking shared artifact...".to_string()],
                stderr_lines: vec![],
            }),
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

/// Test that CheckSerializationResult handles up-to-date status for shared artifacts
#[test]
fn test_shared_check_serialization_result_up_to_date() {
    let model = make_test_model_with_shared();

    // Simulate successful check result indicating up-to-date
    let (new_model, effect) = update(
        model,
        Message::CheckSerializationResult {
            artifact_index: 0,
            status: ArtifactStatus::UpToDate,
            result: Ok(ScriptOutput::default()),
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

/// Test that CheckSerializationResult sets NeedsGeneration when check indicates it
/// Note: Multi-target aggregation now happens in background handler,
/// so update() receives a single aggregated status
#[test]
fn test_shared_check_serialization_result_multi_target() {
    let model = make_test_model_with_shared();

    // Simulate aggregated result where any target needs generation
    // (background handler would have done: any NeedsGeneration => NeedsGeneration)
    let (new_model, effect) = update(
        model,
        Message::CheckSerializationResult {
            artifact_index: 0,
            status: ArtifactStatus::NeedsGeneration,
            result: Ok(ScriptOutput::default()),
        },
    );

    // Status should transition to NeedsGeneration
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
        runs: Vec::new(),
        selected_generator: None,
    };

    let model = Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: Step::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
        generate_queue: Default::default(),
        active_prompt: None,
        last_esc_at: None,
        pipeline_queue: Default::default(),
        in_flight: None,
    };

    // Press Enter on shared artifact
    let (new_model, effect) = update(model, Message::Key(KeyEvent::enter()));

    // Should stay on the artifact list and surface the prompt inline.
    assert!(
        matches!(new_model.screen, Screen::ArtifactList),
        "Single generator + prompts should stay on ArtifactList, got {:?}",
        new_model.screen
    );
    assert!(
        new_model.active_prompt.is_some(),
        "expected active_prompt to be set for inline prompt collection"
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
        runs: Vec::new(),
        selected_generator: None,
    };

    let model = Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: Step::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
        generate_queue: Default::default(),
        active_prompt: None,
        last_esc_at: None,
        pipeline_queue: Default::default(),
        in_flight: None,
    };

    // Press Enter on shared artifact
    let (new_model, effect) = update(model, Message::Key(KeyEvent::enter()));

    // Generation now happens in-place — screen stays on the list and the
    // shared entry's status flips to Generating.
    assert!(matches!(new_model.screen, Screen::ArtifactList));
    assert!(
        matches!(new_model.entries[0].status(), ArtifactStatus::Generating(_)),
        "Single generator without prompts should mark the shared entry Generating, got {:?}",
        new_model.entries[0].status()
    );

    // Effect should be RunGenerator with TargetSpec::Multi
    assert!(
        matches!(effect, Effect::RunGenerator { ref target_spec, .. } if matches!(target_spec, TargetSpec::Multi { .. })),
        "Expected RunGenerator effect with TargetSpec::Multi, got {:?}",
        effect
    );
}

/// Test that multiple generators shows selection dialog
#[test]
fn test_multiple_generators_shows_dialog() {
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
        runs: Vec::new(),
        selected_generator: None,
    };

    let model = Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: Step::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
        generate_queue: Default::default(),
        active_prompt: None,
        last_esc_at: None,
        pipeline_queue: Default::default(),
        in_flight: None,
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
        runs: Vec::new(),
        selected_generator: None,
    };

    let model = Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: Step::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
        generate_queue: Default::default(),
        active_prompt: None,
        last_esc_at: None,
        pipeline_queue: Default::default(),
        in_flight: None,
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
        runs: Vec::new(),
        selected_generator: None,
    };

    Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Shared(shared_entry)],
        selected_index: 0,
        selected_log_step: Step::default(),
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

/// Acceptance criterion: generating the same artifact twice produces two
/// separate run buckets, preserving the logs of the first run alongside the
/// second.
#[test]
fn test_regeneration_produces_distinct_run_buckets() {
    // Run 1: initial check for a pending artifact.
    let mut model = make_test_model();
    let _ = init(&mut model);
    assert_eq!(
        model.entries[0].runs().len(),
        1,
        "init should seed one run per pending entry"
    );

    // Simulate the check reporting UpToDate with some captured output so the
    // first run carries data we can later recognise.
    let (model, _) = update(
        model,
        Message::CheckSerializationResult {
            artifact_index: 0,
            status: ArtifactStatus::UpToDate,
            result: Ok(ScriptOutput {
                stdout_lines: vec!["run-one-check".to_string()],
                stderr_lines: vec![],
            }),
        },
    );
    assert_eq!(model.entries[0].runs().len(), 1);
    let run_one_check_len = model.entries[0].runs()[0].step_logs.check.len();
    assert!(run_one_check_len > 0, "first run should have check logs");

    // Run 2: user triggers regeneration on the now-UpToDate artifact.
    let (model, _) = start_generation_for_selected_internal(model, 0);
    assert_eq!(
        model.entries[0].runs().len(),
        2,
        "regenerating an UpToDate artifact should start a second run"
    );

    // The second run starts empty; the first run's logs are still intact.
    let run_two = &model.entries[0].runs()[1];
    assert!(
        run_two.step_logs.check.is_empty()
            && run_two.step_logs.generate.is_empty()
            && run_two.step_logs.serialize.is_empty(),
        "fresh run should begin with empty logs"
    );
    assert_eq!(
        model.entries[0].runs()[0].step_logs.check.len(),
        run_one_check_len,
        "first run's logs must survive into subsequent runs"
    );
}

/// When a user triggers generation on a NeedsGeneration artifact (no
/// intervening regeneration prompt), we stay inside the run that init seeded.
#[test]
fn test_first_time_generation_continues_initial_run() {
    let mut model = make_test_model();
    let _ = init(&mut model);
    // Check came back needing generation.
    let (model, _) = update(
        model,
        Message::CheckSerializationResult {
            artifact_index: 0,
            status: ArtifactStatus::NeedsGeneration,
            result: Ok(ScriptOutput::default()),
        },
    );

    let (model, _) = start_generation_for_selected_internal(model, 0);
    assert_eq!(
        model.entries[0].runs().len(),
        1,
        "first-time generation should stay in the run seeded by init"
    );
}

// === 'a' generate-all keybind ===

/// Build a model with one of each status so a single `a` keystroke exercises
/// every partition arm: skip UpToDate, dispatch NeedsGeneration immediately,
/// queue Pending until its check resolves.
fn make_mixed_status_model() -> Model {
    let pending = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-pending".to_string(),
        },
        artifact: make_test_artifact("pending-art", vec![]),
        status: ArtifactStatus::Pending,
        runs: Vec::new(),
    };
    let needs_gen = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-needs".to_string(),
        },
        artifact: make_test_artifact("needs-gen-art", vec![]),
        status: ArtifactStatus::NeedsGeneration,
        runs: Vec::new(),
    };
    let up_to_date = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-up".to_string(),
        },
        artifact: make_test_artifact("up-to-date-art", vec![]),
        status: ArtifactStatus::UpToDate,
        runs: Vec::new(),
    };

    Model {
        screen: Screen::ArtifactList,
        entries: vec![
            ListEntry::Single(pending),
            ListEntry::Single(needs_gen),
            ListEntry::Single(up_to_date),
        ],
        selected_index: 0,
        selected_log_step: Step::default(),
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

#[test]
fn test_a_dispatches_needs_generation_and_queues_pending() {
    let model = make_mixed_status_model();
    let (new_model, effect) = update(model, Message::Key(KeyEvent::char('a')));

    // Pending → queued; UpToDate → not queued; NeedsGeneration dispatches.
    assert_eq!(
        new_model.generate_queue,
        std::collections::HashSet::from([0]),
        "only the Pending entry should be queued"
    );

    match effect {
        Effect::RunGenerator { artifact_index, .. } => {
            assert_eq!(artifact_index, 1, "NeedsGeneration entry should dispatch");
        }
        other => panic!(
            "expected single RunGenerator effect for NeedsGeneration, got {:?}",
            other
        ),
    }

    // Screen stays on the artifact list — `a` runs in the background.
    assert!(matches!(new_model.screen, Screen::ArtifactList));
}

#[test]
fn test_a_with_only_up_to_date_does_nothing() {
    let mut model = make_mixed_status_model();
    // Set every entry to UpToDate so `a` should be a no-op.
    for entry in &mut model.entries {
        *entry.status_mut() = ArtifactStatus::UpToDate;
    }

    let (new_model, effect) = update(model, Message::Key(KeyEvent::char('a')));

    assert!(
        new_model.generate_queue.is_empty(),
        "no entry should be queued when all are up to date"
    );
    assert!(
        effect.is_none(),
        "no generation effect when all entries are up to date, got {:?}",
        effect
    );
}

#[test]
fn test_a_skips_needs_generation_with_prompts() {
    // Single artifact with prompts → cannot dispatch directly; falls into the
    // queue so the future inline-prompt flow can pick it up.
    let entry = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine".to_string(),
        },
        artifact: make_test_artifact("with-prompts", vec!["passphrase"]),
        status: ArtifactStatus::NeedsGeneration,
        runs: Vec::new(),
    };
    let model = Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Single(entry)],
        selected_index: 0,
        selected_log_step: Step::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
        generate_queue: Default::default(),
        active_prompt: None,
        last_esc_at: None,
        pipeline_queue: Default::default(),
        in_flight: None,
    };

    let (new_model, effect) = update(model, Message::Key(KeyEvent::char('a')));

    assert_eq!(
        new_model.generate_queue,
        std::collections::HashSet::from([0]),
        "prompted NeedsGeneration entry should be queued, not dispatched"
    );
    assert!(
        effect.is_none(),
        "no generator should fire when prompts are still pending, got {:?}",
        effect
    );
}

#[test]
fn test_check_result_drains_queue_on_needs_generation() {
    // Press 'a': entry 0 (Pending) is queued, entry 1 (NeedsGeneration) goes
    // straight onto the gen→ser pipeline and is dispatched (in_flight = 1).
    // When entry 0's check resolves to NeedsGeneration the queue drains it
    // onto the pipeline, but the effect is None until 1 finishes — that's
    // the whole point of the pipelined order.
    let model = make_mixed_status_model();
    let (model, _) = update(model, Message::Key(KeyEvent::char('a')));
    assert!(model.generate_queue.contains(&0));
    assert_eq!(model.in_flight, Some(1));

    let (new_model, effect) = update(
        model,
        Message::CheckSerializationResult {
            artifact_index: 0,
            status: ArtifactStatus::NeedsGeneration,
            result: Ok(ScriptOutput::default()),
        },
    );

    assert!(
        !new_model.generate_queue.contains(&0),
        "queue should drain when check resolves to NeedsGeneration"
    );
    assert!(
        effect.is_none(),
        "pipeline already in flight for entry 1 — entry 0's RunGenerator must wait, got {:?}",
        effect
    );
    assert_eq!(
        new_model.pipeline_queue.len(),
        1,
        "entry 0's RunGenerator should be queued behind the in-flight entry 1"
    );
    assert!(
        matches!(
            new_model.pipeline_queue.front(),
            Some(Effect::RunGenerator {
                artifact_index: 0,
                ..
            })
        ),
        "expected entry 0's RunGenerator at the head of the pipeline queue"
    );
}

#[test]
fn test_check_result_drops_up_to_date_silently() {
    // Pending entry queued by 'a' resolves UpToDate → drop from queue, no
    // generator dispatched.
    let model = make_mixed_status_model();
    let (model, _) = update(model, Message::Key(KeyEvent::char('a')));
    assert!(model.generate_queue.contains(&0));

    let (new_model, effect) = update(
        model,
        Message::CheckSerializationResult {
            artifact_index: 0,
            status: ArtifactStatus::UpToDate,
            result: Ok(ScriptOutput::default()),
        },
    );

    assert!(
        !new_model.generate_queue.contains(&0),
        "queue should drop UpToDate entry silently"
    );
    assert!(
        effect.is_none(),
        "no effect should fire for UpToDate result, got {:?}",
        effect
    );
}

#[test]
fn test_pipeline_dispatches_one_at_a_time_then_advances_on_serialize() {
    // Three NeedsGeneration entries (no prompts) — pressing 'a' should
    // dispatch only the first via Effect::RunGenerator and queue the rest
    // in pipeline_queue. SerializeFinished for the in-flight entry then
    // pops the next from the pipeline. Locks in the gen0→ser0→gen1→ser1
    // ordering.
    let entries: Vec<ListEntry> = (0..3)
        .map(|i| {
            ListEntry::Single(ArtifactEntry {
                target_type: TargetType::NixOS {
                    machine: format!("m{}", i),
                },
                artifact: make_test_artifact(&format!("art{}", i), vec![]),
                status: ArtifactStatus::NeedsGeneration,
                runs: Vec::new(),
            })
        })
        .collect();
    let model = Model {
        screen: Screen::ArtifactList,
        entries,
        selected_index: 0,
        selected_log_step: Step::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
        generate_queue: Default::default(),
        active_prompt: None,
        last_esc_at: None,
        pipeline_queue: Default::default(),
        in_flight: None,
    };

    let (model, effect) = update(model, Message::Key(KeyEvent::char('a')));

    assert!(
        matches!(
            effect,
            Effect::RunGenerator {
                artifact_index: 0,
                ..
            }
        ),
        "first NeedsGeneration entry should dispatch immediately, got {:?}",
        effect
    );
    assert_eq!(model.in_flight, Some(0));
    assert_eq!(
        model.pipeline_queue.len(),
        2,
        "remaining two entries should be queued behind the in-flight one"
    );

    // Serialize for entry 0 finishes — pipeline must advance to entry 1.
    let (model, effect) = update(
        model,
        Message::SerializeFinished {
            artifact_index: 0,
            result: Ok(ScriptOutput::default()),
        },
    );

    assert!(
        matches!(
            effect,
            Effect::RunGenerator {
                artifact_index: 1,
                ..
            }
        ),
        "pipeline should advance to entry 1 after entry 0 serializes, got {:?}",
        effect
    );
    assert_eq!(model.in_flight, Some(1));
    assert_eq!(model.pipeline_queue.len(), 1);

    // And again — entry 2 is the last.
    let (model, effect) = update(
        model,
        Message::SerializeFinished {
            artifact_index: 1,
            result: Ok(ScriptOutput::default()),
        },
    );

    assert!(
        matches!(
            effect,
            Effect::RunGenerator {
                artifact_index: 2,
                ..
            }
        ),
        "pipeline should advance to entry 2 after entry 1 serializes, got {:?}",
        effect
    );
    assert_eq!(model.in_flight, Some(2));
    assert!(model.pipeline_queue.is_empty());

    // Final serialize → pipeline drained, no further effect.
    let (model, effect) = update(
        model,
        Message::SerializeFinished {
            artifact_index: 2,
            result: Ok(ScriptOutput::default()),
        },
    );
    assert!(
        effect.is_none(),
        "no effect once the pipeline is drained, got {:?}",
        effect
    );
    assert_eq!(model.in_flight, None);
}

#[test]
fn test_check_result_does_not_dispatch_if_not_queued() {
    // Without prior 'a' the check result should never spawn a generator,
    // even when status flips to NeedsGeneration.
    let model = make_mixed_status_model();
    assert!(model.generate_queue.is_empty());

    let (_, effect) = update(
        model,
        Message::CheckSerializationResult {
            artifact_index: 0,
            status: ArtifactStatus::NeedsGeneration,
            result: Ok(ScriptOutput::default()),
        },
    );

    assert!(
        effect.is_none(),
        "no auto-dispatch when entry was not queued by 'a', got {:?}",
        effect
    );
}

#[test]
fn test_generator_finished_works_from_artifact_list_screen() {
    // Sanity check: GeneratorFinished is processed while the user is on
    // Screen::ArtifactList (which is now the only place generation runs from).
    let mut model = make_mixed_status_model();
    *model.entries[1].status_mut() = ArtifactStatus::NeedsGeneration;
    assert!(matches!(model.screen, Screen::ArtifactList));

    let (new_model, effect) = update(
        model,
        Message::GeneratorFinished {
            artifact_index: 1,
            result: Ok(ScriptOutput {
                stdout_lines: vec!["generated".to_string()],
                stderr_lines: vec![],
            }),
        },
    );

    // Logs were appended and a Serialize follow-up was emitted.
    assert!(matches!(
        effect,
        Effect::Serialize {
            artifact_index: 1,
            ..
        }
    ));
    assert!(matches!(new_model.screen, Screen::ArtifactList));
    assert!(
        !new_model.entries[1].step_logs().generate.is_empty(),
        "generator stdout should have been logged"
    );
}

// === Inline prompt: 'a' flow integration ===

/// Build a model with two prompt-bearing NeedsGeneration entries so the 'a'
/// flow has a non-trivial prompt queue to advance through.
fn make_dual_prompt_model() -> Model {
    let entry_a = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-a".to_string(),
        },
        artifact: make_test_artifact("artifact-a", vec!["secret"]),
        status: ArtifactStatus::NeedsGeneration,
        runs: Vec::new(),
    };
    let entry_b = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-b".to_string(),
        },
        artifact: make_test_artifact("artifact-b", vec!["secret"]),
        status: ArtifactStatus::NeedsGeneration,
        runs: Vec::new(),
    };

    Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Single(entry_a), ListEntry::Single(entry_b)],
        selected_index: 0,
        selected_log_step: Step::default(),
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

#[test]
fn test_a_flow_sets_active_prompt_for_first_queued_prompt_bearing() {
    // Both entries are NeedsGeneration with prompts; 'a' enqueues both and
    // surfaces the first inline.
    let model = make_dual_prompt_model();
    let (new_model, effect) = update(model, Message::Key(KeyEvent::char('a')));

    assert_eq!(
        new_model.generate_queue,
        std::collections::HashSet::from([0, 1]),
        "both prompt-bearing entries should be queued"
    );
    let active = new_model
        .active_prompt
        .as_ref()
        .expect("active_prompt should surface the first queued prompt-bearing entry");
    assert_eq!(
        active.artifact_index, 0,
        "lowest-index queued entry wins for stable ordering"
    );
    assert!(
        effect.is_none(),
        "no generator dispatches until the prompt is submitted"
    );
}

#[test]
fn test_prompt_submission_advances_to_next_queued_prompt() {
    // After 'a' opens prompt for entry 0, submitting it should dispatch
    // RunGenerator for 0, drop it from the queue, and advance to entry 1.
    let model = make_dual_prompt_model();
    let (model, _) = update(model, Message::Key(KeyEvent::char('a')));
    assert_eq!(model.active_prompt.as_ref().unwrap().artifact_index, 0);

    // Type a value and submit.
    let (model, _) = update(model, Message::Key(KeyEvent::char('x')));
    let (new_model, effect) = update(model, Message::Key(KeyEvent::enter()));

    // Generator dispatches for the just-submitted entry.
    assert!(
        matches!(
            effect,
            Effect::RunGenerator {
                artifact_index: 0,
                ..
            }
        ),
        "submission should dispatch generator for the entry whose prompt was completed, got {:?}",
        effect
    );

    // Queue drops 0 and active_prompt advances to 1.
    assert_eq!(
        new_model.generate_queue,
        std::collections::HashSet::from([1]),
        "submitted entry must leave the queue"
    );
    let next = new_model
        .active_prompt
        .as_ref()
        .expect("active_prompt should advance to the next queued entry");
    assert_eq!(next.artifact_index, 1);
}

#[test]
fn test_prompt_submission_clears_active_prompt_when_queue_empty() {
    // Single-Enter on a prompt-bearing artifact: nothing queued, so submission
    // dispatches and clears active_prompt back to plain log view.
    let model = make_test_model();
    let (model, _) = update(model, Message::Key(KeyEvent::enter()));
    assert!(model.active_prompt.is_some());
    assert!(model.generate_queue.is_empty());

    let (model, _) = update(model, Message::Key(KeyEvent::char('s')));
    let (new_model, effect) = update(model, Message::Key(KeyEvent::enter()));

    assert!(matches!(effect, Effect::RunGenerator { .. }));
    assert!(
        new_model.active_prompt.is_none(),
        "single-Enter flow clears active_prompt after submission"
    );
    assert!(matches!(new_model.screen, Screen::ArtifactList));
}

#[test]
fn test_prompt_esc_during_a_flow_skips_to_next_queued_artifact() {
    // 'a' opens prompt for entry 0 with entry 1 also queued. Esc must skip
    // entry 0 (drop from queue, no generator), advance to entry 1's prompt,
    // and leave entry 0's status untouched.
    let model = make_dual_prompt_model();
    let (model, _) = update(model, Message::Key(KeyEvent::char('a')));
    assert_eq!(model.active_prompt.as_ref().unwrap().artifact_index, 0);
    assert_eq!(model.generate_queue.len(), 2);

    let (new_model, effect) = update(model, Message::Key(KeyEvent::esc()));

    assert!(
        effect.is_none(),
        "skip must not dispatch a generator, got {:?}",
        effect
    );
    assert!(
        !new_model.generate_queue.contains(&0),
        "skipped entry must leave the queue"
    );
    let next = new_model
        .active_prompt
        .as_ref()
        .expect("active_prompt should advance to the next queued entry after skip");
    assert_eq!(next.artifact_index, 1);
    assert!(
        matches!(
            new_model.entries[0].status(),
            ArtifactStatus::NeedsGeneration
        ),
        "skipped entry retains NeedsGeneration — no Failed mark, no dispatch"
    );
}

#[test]
fn test_prompt_esc_on_last_queued_artifact_clears_prompt() {
    // 'a' run with a single prompt-bearing artifact: Esc skips, leaving the
    // queue empty and the prompt cleared (right pane reverts to logs).
    let entry = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine".to_string(),
        },
        artifact: make_test_artifact("only-one", vec!["secret"]),
        status: ArtifactStatus::NeedsGeneration,
        runs: Vec::new(),
    };
    let model = Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Single(entry)],
        selected_index: 0,
        selected_log_step: Step::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
        generate_queue: Default::default(),
        active_prompt: None,
        last_esc_at: None,
        pipeline_queue: Default::default(),
        in_flight: None,
    };
    let (model, _) = update(model, Message::Key(KeyEvent::char('a')));
    assert_eq!(model.active_prompt.as_ref().unwrap().artifact_index, 0);
    assert_eq!(model.generate_queue.len(), 1);

    let (new_model, effect) = update(model, Message::Key(KeyEvent::esc()));

    assert!(effect.is_none());
    assert!(new_model.active_prompt.is_none());
    assert!(new_model.generate_queue.is_empty());
}

#[test]
fn test_check_resolves_to_prompt_bearing_surfaces_active_prompt() {
    // Pending entry queued by 'a'; when its check resolves to NeedsGeneration
    // with prompts, the inline prompt should surface (no other prompt active).
    let mut model = make_dual_prompt_model();
    // Reset to Pending so check_serialization is meaningful.
    *model.entries[0].status_mut() = ArtifactStatus::Pending;
    *model.entries[1].status_mut() = ArtifactStatus::Pending;

    let (model, _) = update(model, Message::Key(KeyEvent::char('a')));
    assert_eq!(model.generate_queue.len(), 2);
    assert!(
        model.active_prompt.is_none(),
        "no prompt yet — both entries are still Pending awaiting check"
    );

    let (new_model, _) = update(
        model,
        Message::CheckSerializationResult {
            artifact_index: 0,
            status: ArtifactStatus::NeedsGeneration,
            result: Ok(ScriptOutput::default()),
        },
    );

    let active = new_model
        .active_prompt
        .as_ref()
        .expect("inline prompt should surface once a queued entry resolves to NeedsGeneration");
    assert_eq!(active.artifact_index, 0);
    assert!(
        new_model.generate_queue.contains(&0),
        "entry stays queued while its prompt is being collected"
    );
}

#[test]
fn test_navigation_keys_are_swallowed_while_prompt_active() {
    // With active_prompt set, j/k must not navigate the artifact list — they
    // are typed into the prompt buffer.
    let mut model = make_test_model();
    model.active_prompt = Some(make_active_prompt("", InputMode::Line));

    let (model, _) = update(model, Message::Key(KeyEvent::char('j')));

    assert_eq!(
        model.selected_index, 0,
        "j should not navigate when active_prompt is set"
    );
    assert_eq!(model.active_prompt.as_ref().unwrap().buffer, "j");
}

#[test]
fn test_serialize_finished_does_not_kick_user_off_unrelated_screen() {
    // While the 'a' flow is running, the user may navigate to the
    // chronological log of a different artifact. A SerializeFinished arriving
    // for some other artifact must not yank them back to the artifact list.
    let mut model = make_mixed_status_model();
    *model.entries[1].status_mut() = ArtifactStatus::NeedsGeneration;
    model.screen = Screen::ChronologicalLog(ChronologicalLogState::new(
        2,
        "up-to-date-art".to_string(),
        0,
    ));

    let (new_model, _) = update(
        model,
        Message::SerializeFinished {
            artifact_index: 1,
            result: Ok(ScriptOutput::default()),
        },
    );

    assert!(
        matches!(new_model.screen, Screen::ChronologicalLog(_)),
        "user should remain on the log screen they navigated to"
    );
    assert_eq!(
        new_model.entries[1].status(),
        &ArtifactStatus::UpToDate,
        "background result still updates status"
    );
}

// === Cancel-queue: soft-cancel of the 'a' generate-all flow ===

#[test]
fn test_cancel_queue_clears_state_and_returns_cancel_effect() {
    // Mid-'a': two prompt-bearing entries queued, the first surfaced as the
    // active inline prompt. cancel_queue must clear both the queue and the
    // prompt, force the artifact list, and emit Effect::CancelQueue for the
    // runtime to drain the background FIFO.
    let model = make_dual_prompt_model();
    let (model, _) = update(model, Message::Key(KeyEvent::char('a')));
    assert_eq!(model.generate_queue.len(), 2);
    assert!(model.active_prompt.is_some());

    let (new_model, effect) = super::cancel_queue(model);

    assert!(matches!(effect, Effect::CancelQueue));
    assert!(
        new_model.generate_queue.is_empty(),
        "queue must be empty after cancel"
    );
    assert!(
        new_model.active_prompt.is_none(),
        "active inline prompt must clear so the right pane reverts to logs"
    );
    assert!(matches!(new_model.screen, Screen::ArtifactList));
}

#[test]
fn test_cancel_queue_reverts_pending_entries_to_needs_generation() {
    // Pending entries (their CheckSerialization may have been queued behind
    // the cancel-target generators) revert to NeedsGeneration so the user
    // sees a stable, retriggerable state. Other statuses are left alone.
    let mut model = make_mixed_status_model();
    // Layout: [Pending, NeedsGeneration, UpToDate]. Add a Generating entry
    // to confirm in-flight statuses are not touched.
    let generating = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-gen".to_string(),
        },
        artifact: make_test_artifact("running", vec![]),
        status: ArtifactStatus::Generating(GeneratingSubstate::default()),
        runs: Vec::new(),
    };
    model.entries.push(ListEntry::Single(generating));

    let (new_model, _) = super::cancel_queue(model);

    assert_eq!(
        new_model.entries[0].status(),
        &ArtifactStatus::NeedsGeneration,
        "Pending → NeedsGeneration after cancel"
    );
    assert_eq!(
        new_model.entries[1].status(),
        &ArtifactStatus::NeedsGeneration,
        "NeedsGeneration entries are unchanged"
    );
    assert_eq!(
        new_model.entries[2].status(),
        &ArtifactStatus::UpToDate,
        "UpToDate entries are unchanged — no destructive resets"
    );
    assert!(
        matches!(new_model.entries[3].status(), ArtifactStatus::Generating(_)),
        "model-side cancel_queue does not transition Generating; the runtime kills \
         the bwrap process group and the status flips on GeneratorCancelled"
    );
}

#[test]
fn test_cancel_queue_when_idle_is_a_noop_aside_from_effect() {
    // No queue, no active prompt, no Pending entries. cancel_queue should
    // still return Effect::CancelQueue (the runtime forwards an empty drain),
    // and the model should be byte-identical aside from the screen forced to
    // ArtifactList.
    let mut model = make_mixed_status_model();
    *model.entries[0].status_mut() = ArtifactStatus::UpToDate;
    *model.entries[1].status_mut() = ArtifactStatus::UpToDate;
    *model.entries[2].status_mut() = ArtifactStatus::UpToDate;
    model.screen = Screen::ArtifactList;

    let (new_model, effect) = super::cancel_queue(model);

    assert!(matches!(effect, Effect::CancelQueue));
    assert!(new_model.generate_queue.is_empty());
    assert!(new_model.active_prompt.is_none());
    for entry in &new_model.entries {
        assert_eq!(entry.status(), &ArtifactStatus::UpToDate);
    }
}

#[test]
fn test_cancel_queue_dismisses_modal_screens() {
    // Cancel during a generator-selection or confirm-regenerate dialog must
    // dismiss the dialog and return the user to the artifact list — those
    // dialogs were started by the same flow being cancelled.
    let mut model = make_mixed_status_model();
    model.screen = Screen::ConfirmRegenerate(ConfirmRegenerateState {
        artifact_index: 2,
        artifact_name: "up-to-date-art".to_string(),
        affected_targets: vec!["machine-up".to_string()],
        leave_selected: true,
    });

    let (new_model, _) = super::cancel_queue(model);

    assert!(matches!(new_model.screen, Screen::ArtifactList));
}

// === Esc-Esc cancel chord (universal, 500ms window) ===

#[test]
fn test_esc_chord_during_a_flow_cancels_queue() {
    // 'a' flow with two prompt-bearing artifacts queued. First Esc skips
    // entry 0 and surfaces entry 1's prompt (existing per-context behavior).
    // Second Esc within 500ms fires the chord: queue cleared, prompt cleared,
    // CancelQueue effect emitted, screen forced to ArtifactList.
    let model = make_dual_prompt_model();
    let (model, _) = update(model, Message::Key(KeyEvent::char('a')));
    assert_eq!(model.generate_queue.len(), 2);

    let (model, _) = update(model, Message::Key(KeyEvent::esc()));
    assert!(
        model.last_esc_at.is_some(),
        "first Esc must seed the chord timer"
    );
    // After first Esc: entry 0 skipped, entry 1's prompt surfaced.
    assert_eq!(model.active_prompt.as_ref().unwrap().artifact_index, 1);
    assert_eq!(model.generate_queue.len(), 1);

    // Second Esc, immediately — well within the 500ms window.
    let (new_model, effect) = update(model, Message::Key(KeyEvent::esc()));

    assert!(matches!(effect, Effect::CancelQueue));
    assert!(new_model.generate_queue.is_empty());
    assert!(new_model.active_prompt.is_none());
    assert!(matches!(new_model.screen, Screen::ArtifactList));
    assert!(
        new_model.last_esc_at.is_none(),
        "chord timer cleared after firing"
    );
}

#[test]
fn test_esc_chord_in_plain_view_emits_cancel_effect() {
    // Plain artifact list, no queue, no active prompt. Two Esc presses
    // within 500ms still dispatch CancelQueue (the runtime drains an empty
    // FIFO — a no-op functionally) and return to the list (already there).
    let model = make_test_model();
    assert!(model.generate_queue.is_empty());
    assert!(model.active_prompt.is_none());

    let (model, effect) = update(model, Message::Key(KeyEvent::esc()));
    assert!(effect.is_none(), "first Esc on plain list is a no-op");
    assert!(model.last_esc_at.is_some());

    let (new_model, effect) = update(model, Message::Key(KeyEvent::esc()));

    assert!(matches!(effect, Effect::CancelQueue));
    assert!(matches!(new_model.screen, Screen::ArtifactList));
    assert!(new_model.generate_queue.is_empty());
    assert!(new_model.last_esc_at.is_none());
}

#[test]
fn test_esc_chord_times_out_after_window() {
    // Two Esc presses >500ms apart must behave as two independent single-Esc
    // events. We can't sleep in tests, so manually backdate `last_esc_at`
    // past the chord window after the first Esc.
    let model = make_test_model();
    let (mut model, _) = update(model, Message::Key(KeyEvent::esc()));
    let first_ts = model.last_esc_at.expect("first Esc seeds the timer");

    // Backdate past the window — second Esc must be treated as a fresh first.
    model.last_esc_at = Some(first_ts - Duration::from_millis(600));

    let (new_model, effect) = update(model, Message::Key(KeyEvent::esc()));

    assert!(
        effect.is_none(),
        "timed-out chord must NOT fire CancelQueue, got {:?}",
        effect
    );
    assert!(
        new_model.last_esc_at.is_some(),
        "second Esc seeds a fresh chord timer instead of firing"
    );
    assert!(matches!(new_model.screen, Screen::ArtifactList));
}

#[test]
fn test_non_esc_key_resets_chord_state() {
    // Esc → 'j' → Esc (within 500ms total) must NOT fire the chord — the
    // intervening non-Esc keypress breaks the sequence.
    let model = make_test_model();
    let (model, _) = update(model, Message::Key(KeyEvent::esc()));
    assert!(model.last_esc_at.is_some());

    let (model, _) = update(model, Message::Key(KeyEvent::char('j')));
    assert!(
        model.last_esc_at.is_none(),
        "non-Esc key must clear the chord timer"
    );

    let (new_model, effect) = update(model, Message::Key(KeyEvent::esc()));

    assert!(
        effect.is_none(),
        "second Esc after intervening key must not fire chord"
    );
    assert!(
        new_model.last_esc_at.is_some(),
        "second Esc seeds a fresh timer"
    );
}

#[test]
fn test_tick_does_not_clear_chord_state() {
    // Tick messages arrive every ~50ms during animation; they must not
    // close the chord window or the user's effective window would shrink to
    // a single tick boundary.
    let model = make_test_model();
    let (model, _) = update(model, Message::Key(KeyEvent::esc()));
    let seeded = model.last_esc_at.expect("first Esc seeds the timer");

    let (model, _) = update(model, Message::Tick);
    assert_eq!(
        model.last_esc_at,
        Some(seeded),
        "Tick must leave the chord timer alone"
    );

    let (new_model, effect) = update(model, Message::Key(KeyEvent::esc()));
    assert!(matches!(effect, Effect::CancelQueue));
    assert!(new_model.last_esc_at.is_none());
}
