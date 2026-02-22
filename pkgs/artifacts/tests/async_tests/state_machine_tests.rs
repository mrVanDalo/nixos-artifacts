//! State machine simulation tests for async effect handling.
//!
//! These tests verify the complete state machine transitions using a dual assertion strategy:
//! 1. Commands sent to the background task match expected EffectCommand variants
//! 2. Final Model state correctly reflects async operation results
//!
//! Tests cover full lifecycle transitions:
//! - Pending → CheckRunning → CheckComplete → (NeedsGen | UpToDate)
//! - Pending → Generating → GeneratorFinished → (Success → PendingSerialization → Serializing → Done | Failed)
//! - Shared artifact multi-target flows

use std::collections::{BTreeMap, HashMap};

use artifacts::app::effect::Effect;
use artifacts::app::message::{CheckOutput, GeneratorOutput, Msg, SerializeOutput};
use artifacts::app::model::{
    ArtifactEntry, ArtifactStatus, GeneratingState, GenerationStep, ListEntry, Model, Screen,
    StepLogs, TargetType,
};
use artifacts::app::update::update;
use artifacts::config::backend::BackendConfiguration;
use artifacts::config::make::{ArtifactDef, FileDef, MakeConfiguration, PromptDef};
use artifacts::tui::channels::{EffectCommand, EffectResult};
use serial_test::serial;

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a minimal backend config for testing
#[allow(dead_code)]
fn create_test_backend_config() -> BackendConfiguration {
    BackendConfiguration {
        config: HashMap::new(),
        base_path: std::path::PathBuf::from("."),
        backend_toml: std::path::PathBuf::from("./test.toml"),
    }
}

/// Create a minimal make config for testing
#[allow(dead_code)]
fn create_test_make_config() -> MakeConfiguration {
    MakeConfiguration {
        nixos_map: BTreeMap::new(),
        home_map: BTreeMap::new(),
        nixos_config: BTreeMap::new(),
        home_config: BTreeMap::new(),
        make_base: std::path::PathBuf::from("."),
        make_json: std::path::PathBuf::from("./test.json"),
    }
}

/// Create a test artifact definition
fn create_test_artifact(name: &str, has_prompts: bool) -> ArtifactDef {
    let mut prompts = BTreeMap::new();
    if has_prompts {
        prompts.insert(
            "password".to_string(),
            PromptDef {
                name: "password".to_string(),
                description: Some("Enter password".to_string()),
            },
        );
    }

    let mut files = BTreeMap::new();
    files.insert(
        "key".to_string(),
        FileDef {
            name: "key".to_string(),
            path: Some("/etc/secrets/key".to_string()),
            owner: None,
            group: None,
        },
    );

    ArtifactDef {
        name: name.to_string(),
        description: None,
        shared: false,
        files,
        prompts,
        generator: "/test/generator.sh".to_string(),
        serialization: "test".to_string(),
    }
}

/// Create a test model with a single artifact entry
fn create_test_model(artifact_name: &str, has_prompts: bool) -> Model {
    let artifact = create_test_artifact(artifact_name, has_prompts);
    let entry = ArtifactEntry {
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        artifact: artifact.clone(),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
        exists: false,
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

/// Command tracker for dual assertion strategy
struct CommandTracker {
    commands: Vec<EffectCommand>,
}

impl CommandTracker {
    fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    fn track(&mut self, cmd: EffectCommand) {
        self.commands.push(cmd);
    }

    fn assert_command_count(&self, expected: usize) {
        assert_eq!(
            self.commands.len(),
            expected,
            "Expected {} commands, got {}",
            expected,
            self.commands.len()
        );
    }

    fn assert_command_at(&self, index: usize, expected_name: &str) {
        let cmd = self.commands.get(index).expect("Command should exist");
        let actual = match cmd {
            EffectCommand::CheckSerialization { .. } => "CheckSerialization",
            EffectCommand::RunGenerator { .. } => "RunGenerator",
            EffectCommand::Serialize { .. } => "Serialize",
            EffectCommand::SharedCheckSerialization { .. } => "SharedCheckSerialization",
            EffectCommand::RunSharedGenerator { .. } => "RunSharedGenerator",
            EffectCommand::SharedSerialize { .. } => "SharedSerialize",
        };
        assert_eq!(actual, expected_name, "Command at index {} mismatch", index);
    }

    fn get_artifact_index_at(&self, index: usize) -> usize {
        let cmd = self.commands.get(index).expect("Command should exist");
        match cmd {
            EffectCommand::CheckSerialization { artifact_index, .. } => *artifact_index,
            EffectCommand::RunGenerator { artifact_index, .. } => *artifact_index,
            EffectCommand::Serialize { artifact_index, .. } => *artifact_index,
            EffectCommand::SharedCheckSerialization { artifact_index, .. } => *artifact_index,
            EffectCommand::RunSharedGenerator { artifact_index, .. } => *artifact_index,
            EffectCommand::SharedSerialize { artifact_index, .. } => *artifact_index,
        }
    }
}

/// Convert Effect to EffectCommand for tracking
fn effect_to_command(effect: &Effect) -> Option<EffectCommand> {
    match effect {
        Effect::CheckSerialization {
            artifact_index,
            artifact_name,
            target,
            target_type,
        } => Some(EffectCommand::CheckSerialization {
            artifact_index: *artifact_index,
            artifact_name: artifact_name.clone(),
            target: target.clone(),
            target_type: target_type.context_str().to_string(),
        }),
        Effect::RunGenerator {
            artifact_index,
            artifact_name,
            target,
            target_type,
            prompts,
        } => Some(EffectCommand::RunGenerator {
            artifact_index: *artifact_index,
            artifact_name: artifact_name.clone(),
            target: target.clone(),
            target_type: target_type.context_str().to_string(),
            prompts: prompts.clone(),
        }),
        Effect::Serialize {
            artifact_index,
            artifact_name,
            target,
            target_type,
            ..
        } => Some(EffectCommand::Serialize {
            artifact_index: *artifact_index,
            artifact_name: artifact_name.clone(),
            target: target.clone(),
            target_type: target_type.context_str().to_string(),
        }),
        Effect::SharedCheckSerialization {
            artifact_index,
            artifact_name,
            nixos_targets,
            home_targets,
            ..
        } => Some(EffectCommand::SharedCheckSerialization {
            artifact_index: *artifact_index,
            artifact_name: artifact_name.clone(),
            targets: nixos_targets
                .iter()
                .chain(home_targets.iter())
                .cloned()
                .collect(),
            target_types: std::iter::repeat_n("nixos".to_string(), nixos_targets.len())
                .chain(std::iter::repeat_n("home".to_string(), home_targets.len()))
                .collect(),
        }),
        Effect::RunSharedGenerator {
            artifact_index,
            artifact_name,
            prompts,
            nixos_targets,
            home_targets,
            ..
        } => Some(EffectCommand::RunSharedGenerator {
            artifact_index: *artifact_index,
            artifact_name: artifact_name.clone(),
            machine_targets: nixos_targets.clone(),
            user_targets: home_targets.clone(),
            prompts: prompts.clone(),
        }),
        Effect::SharedSerialize {
            artifact_index,
            artifact_name,
            nixos_targets,
            home_targets,
            ..
        } => Some(EffectCommand::SharedSerialize {
            artifact_index: *artifact_index,
            artifact_name: artifact_name.clone(),
            machine_targets: nixos_targets.clone(),
            user_targets: home_targets.clone(),
        }),
        _ => None,
    }
}

/// Simulate processing effects and tracking commands
fn process_effects_and_track(
    model: &mut Model,
    effect: Effect,
    tracker: &mut CommandTracker,
) -> Vec<EffectResult> {
    let mut results = Vec::new();

    fn process_single_effect(
        _model: &mut Model,
        effect: Effect,
        tracker: &mut CommandTracker,
        _results: &mut Vec<EffectResult>,
    ) {
        if let Some(cmd) = effect_to_command(&effect) {
            tracker.track(cmd);
        }

        // Handle batch effects recursively
        if let Effect::Batch(effects) = effect {
            for e in effects {
                process_single_effect(_model, e, tracker, _results);
            }
        }
    }

    process_single_effect(model, effect, tracker, &mut results);
    results
}

// ============================================================================
// State Machine Transition Tests
// ============================================================================

/// Test: CheckSerialization flow
/// Transition: Pending → CheckComplete → NeedsGeneration
/// Command: CheckSerialization sent
/// Final State: ArtifactStatus::NeedsGeneration
#[test]
#[serial]
fn test_check_serialization_flow_needs_generation() {
    let mut model = create_test_model("test-artifact", false);
    let mut tracker = CommandTracker::new();

    // Initial state should be Pending
    assert_eq!(model.entries[0].status(), &ArtifactStatus::Pending);

    // Simulate check_serialization effect (normally triggered by init)
    let check_effect = Effect::CheckSerialization {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
    };

    process_effects_and_track(&mut model, check_effect, &mut tracker);

    // Dual assertion 1: Command was tracked correctly
    tracker.assert_command_count(1);
    tracker.assert_command_at(0, "CheckSerialization");
    assert_eq!(tracker.get_artifact_index_at(0), 0);

    // Simulate successful check result indicating generation needed
    let check_output = CheckOutput {
        stdout_lines: vec!["Check passed".to_string()],
        stderr_lines: vec![],
    };
    let (new_model, _) = update(
        model,
        Msg::CheckSerializationResult {
            artifact_index: 0,
            needs_generation: true,
            exists: false,
            result: Ok(()),
            output: Some(check_output),
        },
    );

    // Dual assertion 2: Final state reflects successful check
    assert_eq!(
        new_model.entries[0].status(),
        &ArtifactStatus::NeedsGeneration,
        "Status should transition to NeedsGeneration"
    );
}

/// Test: CheckSerialization flow
/// Transition: Pending → CheckComplete → UpToDate
/// Command: CheckSerialization sent
/// Final State: ArtifactStatus::UpToDate
#[test]
#[serial]
fn test_check_serialization_flow_up_to_date() {
    let mut model = create_test_model("test-artifact", false);
    let mut tracker = CommandTracker::new();

    // Initial state
    assert_eq!(model.entries[0].status(), &ArtifactStatus::Pending);

    // Simulate check_serialization effect
    let check_effect = Effect::CheckSerialization {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
    };

    process_effects_and_track(&mut model, check_effect, &mut tracker);

    // Dual assertion 1: Command tracked
    tracker.assert_command_count(1);
    tracker.assert_command_at(0, "CheckSerialization");

    // Simulate successful check result indicating no generation needed
    let (new_model, _) = update(
        model,
        Msg::CheckSerializationResult {
            artifact_index: 0,
            needs_generation: false,
            exists: true,
            result: Ok(()),
            output: None,
        },
    );

    // Dual assertion 2: Final state is UpToDate
    assert_eq!(
        new_model.entries[0].status(),
        &ArtifactStatus::UpToDate,
        "Status should transition to UpToDate"
    );
}

/// Test: Generator flow with success
/// Transition: Pending → Generating → GeneratorFinished → Success → PendingSerialization → Serializing → Done
/// Commands: RunGenerator, Serialize
/// Final State: ArtifactStatus::UpToDate
#[test]
#[serial]
fn test_generator_flow_success() {
    let mut model = create_test_model("test-artifact", false);
    let mut tracker = CommandTracker::new();

    // Start at Pending state
    assert_eq!(model.entries[0].status(), &ArtifactStatus::Pending);

    // Set screen to Generating (normally done by start_generation_for_selected)
    model.screen = Screen::Generating(GeneratingState {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        step: GenerationStep::RunningGenerator,
        log_lines: vec![],
        exists: false,
    });

    // Simulate generation effect
    let generator_effect = Effect::RunGenerator {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        prompts: HashMap::new(),
    };

    process_effects_and_track(&mut model, generator_effect, &mut tracker);

    // Dual assertion 1: RunGenerator command tracked
    tracker.assert_command_count(1);
    tracker.assert_command_at(0, "RunGenerator");
    assert_eq!(tracker.get_artifact_index_at(0), 0);

    // Simulate successful generator result
    let generator_output = GeneratorOutput {
        stdout_lines: vec!["Generated key file".to_string()],
        stderr_lines: vec![],
        files_generated: 1,
    };
    let (model_after_gen, serialize_effect) = update(
        model.clone(),
        Msg::GeneratorFinished {
            artifact_index: 0,
            result: Ok(generator_output),
        },
    );

    // After generator, we should have moved to serialization step
    if let Screen::Generating(state) = &model_after_gen.screen {
        assert_eq!(state.step, GenerationStep::Serializing);
    } else {
        panic!(
            "Expected screen to be Generating, got {:?}",
            model_after_gen.screen
        );
    }

    // Process serialize effect
    tracker.commands.clear(); // Clear previous commands
    process_effects_and_track(&mut model, serialize_effect, &mut tracker);
    tracker.assert_command_count(1);
    tracker.assert_command_at(0, "Serialize");

    // Simulate successful serialize result - set screen back to Generating with Serializing step
    let mut model_for_serialize = model_after_gen;
    model_for_serialize.screen = Screen::Generating(GeneratingState {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        step: GenerationStep::Serializing,
        log_lines: vec![],
        exists: false,
    });

    let serialize_output = SerializeOutput {
        stdout_lines: vec!["Serialized to backend".to_string()],
        stderr_lines: vec![],
    };
    let (final_model, _) = update(
        model_for_serialize,
        Msg::SerializeFinished {
            artifact_index: 0,
            result: Ok(serialize_output),
        },
    );

    // Dual assertion 2: Final state is UpToDate
    assert_eq!(
        final_model.entries[0].status(),
        &ArtifactStatus::UpToDate,
        "Status should transition to UpToDate after successful serialize"
    );
}

/// Test: Generator flow with failure
/// Transition: Pending → Generating → GeneratorFinished → Failed
/// Command: RunGenerator
/// Final State: ArtifactStatus::Failed
#[test]
#[serial]
fn test_generator_flow_failure() {
    let mut model = create_test_model("test-artifact", false);
    let mut tracker = CommandTracker::new();

    // Set screen to Generating (normally done by start_generation_for_selected)
    model.screen = Screen::Generating(GeneratingState {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        step: GenerationStep::RunningGenerator,
        log_lines: vec![],
        exists: false,
    });

    // Simulate generation effect
    let generator_effect = Effect::RunGenerator {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        prompts: HashMap::new(),
    };

    process_effects_and_track(&mut model, generator_effect, &mut tracker);

    // Dual assertion 1: Command tracked
    tracker.assert_command_count(1);
    tracker.assert_command_at(0, "RunGenerator");

    // Simulate failed generator result
    let (final_model, _) = update(
        model,
        Msg::GeneratorFinished {
            artifact_index: 0,
            result: Err("Generator script failed with exit code 1".to_string()),
        },
    );

    // Dual assertion 2: Final state is Failed
    match final_model.entries[0].status() {
        ArtifactStatus::Failed {
            error,
            retry_available,
            ..
        } => {
            assert!(error.contains("Generator failed"));
            assert!(*retry_available);
        }
        other => panic!("Expected Failed status, got {:?}", other),
    }
}

/// Test: Serialize flow with failure
/// Transition: Pending → Generating → GeneratorFinished → Success → Serializing → Failed
/// Commands: RunGenerator, Serialize
/// Final State: ArtifactStatus::Failed
#[test]
#[serial]
fn test_serialize_flow_failure() {
    let mut model = create_test_model("test-artifact", false);
    let mut tracker = CommandTracker::new();

    // Set screen to Generating with RunningGenerator step
    model.screen = Screen::Generating(GeneratingState {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        step: GenerationStep::RunningGenerator,
        log_lines: vec![],
        exists: false,
    });

    // Simulate successful generation first
    let generator_output = GeneratorOutput {
        stdout_lines: vec!["Generated".to_string()],
        stderr_lines: vec![],
        files_generated: 1,
    };
    let (model_after_gen, _) = update(
        model.clone(),
        Msg::GeneratorFinished {
            artifact_index: 0,
            result: Ok(generator_output),
        },
    );

    // Process serialize effect
    let serialize_effect = Effect::Serialize {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        out_dir: std::path::PathBuf::from("/tmp"),
    };

    process_effects_and_track(&mut model, serialize_effect, &mut tracker);
    tracker.assert_command_at(0, "Serialize");

    // Set screen to Generating with Serializing step for handle_serialize_finished
    let mut model_for_serialize = model_after_gen;
    model_for_serialize.screen = Screen::Generating(GeneratingState {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        step: GenerationStep::Serializing,
        log_lines: vec![],
        exists: false,
    });

    // Simulate failed serialize result
    let (final_model, _) = update(
        model_for_serialize,
        Msg::SerializeFinished {
            artifact_index: 0,
            result: Err("Serialize script failed".to_string()),
        },
    );

    // Dual assertion: Final state is Failed
    match final_model.entries[0].status() {
        ArtifactStatus::Failed { error, .. } => {
            assert!(error.contains("Serialization failed"));
        }
        other => panic!("Expected Failed status, got {:?}", other),
    }
}

/// Test: CheckSerialization failure handling
/// Transition: Pending → CheckComplete → Failed
/// Command: CheckSerialization
/// Final State: ArtifactStatus::Failed
#[test]
#[serial]
fn test_check_serialization_failure() {
    let model = create_test_model("test-artifact", false);
    let mut tracker = CommandTracker::new();

    // Simulate check effect
    let check_effect = Effect::CheckSerialization {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
    };

    process_effects_and_track(&mut model.clone(), check_effect, &mut tracker);
    tracker.assert_command_at(0, "CheckSerialization");

    // Simulate failed check result
    let (final_model, _) = update(
        model,
        Msg::CheckSerializationResult {
            artifact_index: 0,
            needs_generation: true,
            exists: false,
            result: Err("Check script not found".to_string()),
            output: None,
        },
    );

    // Final state should be Failed with retry available
    match final_model.entries[0].status() {
        ArtifactStatus::Failed {
            error,
            retry_available,
            ..
        } => {
            assert!(error.contains("Check script not found"));
            assert!(*retry_available);
        }
        other => panic!("Expected Failed status, got {:?}", other),
    }
}

/// Test: Batch effect processing
/// Verifies that multiple check commands are tracked in order
#[test]
#[serial]
fn test_batch_effect_processing() {
    // Create model with multiple entries
    let artifact1 = create_test_artifact("artifact-1", false);
    let artifact2 = create_test_artifact("artifact-2", false);

    let entry1 = ArtifactEntry {
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        artifact: artifact1.clone(),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
        exists: false,
    };
    let entry2 = ArtifactEntry {
        target: "machine-two".to_string(),
        target_type: TargetType::Nixos,
        artifact: artifact2.clone(),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
        exists: false,
    };

    let mut model = Model {
        screen: Screen::ArtifactList,
        artifacts: vec![entry1.clone(), entry2.clone()],
        entries: vec![ListEntry::Single(entry1), ListEntry::Single(entry2)],
        selected_index: 0,
        selected_log_step: Default::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    };

    let mut tracker = CommandTracker::new();

    // Create batch effect with multiple checks
    let batch_effect = Effect::Batch(vec![
        Effect::CheckSerialization {
            artifact_index: 0,
            artifact_name: "artifact-1".to_string(),
            target: "machine-one".to_string(),
            target_type: TargetType::Nixos,
        },
        Effect::CheckSerialization {
            artifact_index: 1,
            artifact_name: "artifact-2".to_string(),
            target: "machine-two".to_string(),
            target_type: TargetType::Nixos,
        },
    ]);

    process_effects_and_track(&mut model, batch_effect, &mut tracker);

    // Verify both commands tracked in order
    tracker.assert_command_count(2);
    tracker.assert_command_at(0, "CheckSerialization");
    tracker.assert_command_at(1, "CheckSerialization");
    assert_eq!(tracker.get_artifact_index_at(0), 0);
    assert_eq!(tracker.get_artifact_index_at(1), 1);
}

/// Test: Artifact index preservation through state machine
/// Verifies artifact_index is preserved in all command/result exchanges
#[test]
#[serial]
fn test_artifact_index_preservation() {
    // Test with various artifact indices
    for idx in [0usize, 5, 100, 999] {
        let model = create_test_model("test", false);
        let mut tracker = CommandTracker::new();

        let effect = Effect::CheckSerialization {
            artifact_index: idx,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: TargetType::Nixos,
        };

        process_effects_and_track(&mut model.clone(), effect, &mut tracker);

        assert_eq!(
            tracker.get_artifact_index_at(0),
            idx,
            "artifact_index should be preserved: expected {}, got {}",
            idx,
            tracker.get_artifact_index_at(0)
        );
    }
}

/// Test: Complete end-to-end flow
/// Simulates full lifecycle: Check → Generation → Serialize
/// Commands tracked: CheckSerialization, RunGenerator, Serialize
/// Final State: UpToDate
#[test]
#[serial]
fn test_complete_lifecycle_success() {
    let mut model = create_test_model("test-artifact", false);
    let mut tracker = CommandTracker::new();

    // Step 1: CheckSerialization
    let check_effect = Effect::CheckSerialization {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
    };
    process_effects_and_track(&mut model, check_effect, &mut tracker);

    // Check result says we need generation
    let (model, _) = update(
        model,
        Msg::CheckSerializationResult {
            artifact_index: 0,
            needs_generation: true,
            exists: false,
            result: Ok(()),
            output: None,
        },
    );
    assert_eq!(model.entries[0].status(), &ArtifactStatus::NeedsGeneration);

    // Step 2: RunGenerator (user triggered) - need Generating screen
    let mut model_with_screen = model.clone();
    model_with_screen.screen = Screen::Generating(GeneratingState {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        step: GenerationStep::RunningGenerator,
        log_lines: vec![],
        exists: false,
    });

    let gen_effect = Effect::RunGenerator {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
        prompts: HashMap::new(),
    };
    process_effects_and_track(&mut model.clone(), gen_effect, &mut tracker);

    let gen_output = GeneratorOutput {
        stdout_lines: vec!["Generated".to_string()],
        stderr_lines: vec![],
        files_generated: 1,
    };
    let (model_after_gen, serialize_effect) = update(
        model_with_screen,
        Msg::GeneratorFinished {
            artifact_index: 0,
            result: Ok(gen_output),
        },
    );

    // Step 3: Serialize - need Generating screen with Serializing step
    let mut model_for_serialize = model_after_gen;
    model_for_serialize.screen = Screen::Generating(GeneratingState {
        artifact_index: 0,
        artifact_name: "test-artifact".to_string(),
        step: GenerationStep::Serializing,
        log_lines: vec![],
        exists: false,
    });

    process_effects_and_track(&mut model.clone(), serialize_effect, &mut tracker);

    let serialize_output = SerializeOutput {
        stdout_lines: vec!["Serialized".to_string()],
        stderr_lines: vec![],
    };
    let (final_model, _) = update(
        model_for_serialize,
        Msg::SerializeFinished {
            artifact_index: 0,
            result: Ok(serialize_output),
        },
    );

    // Verify all commands tracked
    tracker.assert_command_count(3);
    tracker.assert_command_at(0, "CheckSerialization");
    tracker.assert_command_at(1, "RunGenerator");
    tracker.assert_command_at(2, "Serialize");

    // Final state should be UpToDate
    assert_eq!(final_model.entries[0].status(), &ArtifactStatus::UpToDate);
}

/// Test: State persistence through failed check
/// When check fails, status becomes Failed with retry_available
#[test]
#[serial]
fn test_retry_available_after_failed_check() {
    let model = create_test_model("test-artifact", false);

    let (final_model, _) = update(
        model,
        Msg::CheckSerializationResult {
            artifact_index: 0,
            needs_generation: true,
            exists: false,
            result: Err("Backend not configured".to_string()),
            output: None,
        },
    );

    match final_model.entries[0].status() {
        ArtifactStatus::Failed {
            retry_available, ..
        } => {
            assert!(
                *retry_available,
                "retry_available should be true for check failures"
            );
        }
        other => panic!("Expected Failed status, got {:?}", other),
    }
}

/// Test: Multiple command types in sequence
/// Verifies command tracking works across different effect types
#[test]
#[serial]
fn test_multiple_command_types_tracked() {
    let model = create_test_model("test-artifact", false);
    let mut tracker = CommandTracker::new();

    // Send all three main command types
    let effects = vec![
        Effect::CheckSerialization {
            artifact_index: 0,
            artifact_name: "check".to_string(),
            target: "m1".to_string(),
            target_type: TargetType::Nixos,
        },
        Effect::RunGenerator {
            artifact_index: 0,
            artifact_name: "gen".to_string(),
            target: "m2".to_string(),
            target_type: TargetType::Nixos,
            prompts: HashMap::new(),
        },
        Effect::Serialize {
            artifact_index: 0,
            artifact_name: "ser".to_string(),
            target: "m3".to_string(),
            target_type: TargetType::Nixos,
            out_dir: std::path::PathBuf::from("/tmp"),
        },
    ];

    for effect in effects {
        process_effects_and_track(&mut model.clone(), effect, &mut tracker);
    }

    // All three command types should be tracked
    tracker.assert_command_count(3);
    tracker.assert_command_at(0, "CheckSerialization");
    tracker.assert_command_at(1, "RunGenerator");
    tracker.assert_command_at(2, "Serialize");
}

/// Test: Empty batch effect
/// Verifies batch with no effects doesn't track any commands
#[test]
#[serial]
fn test_empty_batch_tracks_no_commands() {
    let mut model = create_test_model("test-artifact", false);
    let mut tracker = CommandTracker::new();

    let empty_batch = Effect::Batch(vec![]);
    process_effects_and_track(&mut model, empty_batch, &mut tracker);

    tracker.assert_command_count(0);
}

/// Test: Batch filtering
/// Verifies Effect::None is filtered from batches
#[test]
#[serial]
fn test_batch_filters_none_effects() {
    let mut model = create_test_model("test-artifact", false);
    let mut tracker = CommandTracker::new();

    let batch_with_none = Effect::Batch(vec![
        Effect::None,
        Effect::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target: "m1".to_string(),
            target_type: TargetType::Nixos,
        },
        Effect::None,
        Effect::RunGenerator {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target: "m2".to_string(),
            target_type: TargetType::Nixos,
            prompts: HashMap::new(),
        },
    ]);

    process_effects_and_track(&mut model, batch_with_none, &mut tracker);

    // Only 2 commands should be tracked (None filtered)
    tracker.assert_command_count(2);
}

/// Test: Command variant extraction
/// Verifies all EffectCommand variants can be extracted and tracked
#[test]
#[serial]
fn test_all_command_variants_extractable() {
    let test_effect = Effect::CheckSerialization {
        artifact_index: 42,
        artifact_name: "test-check".to_string(),
        target: "machine-test".to_string(),
        target_type: TargetType::Nixos,
    };

    let cmd = effect_to_command(&test_effect);
    assert!(cmd.is_some());

    if let Some(EffectCommand::CheckSerialization {
        artifact_index,
        artifact_name,
        target,
        target_type,
    }) = cmd
    {
        assert_eq!(artifact_index, 42);
        assert_eq!(artifact_name, "test-check");
        assert_eq!(target, "machine-test");
        assert_eq!(target_type, "nixos");
    } else {
        panic!("Expected CheckSerialization variant");
    }
}

/// Test: Dual assertion strategy demonstration
/// Shows how tests verify both command variant and final state
#[test]
#[serial]
fn test_dual_assertion_strategy_demonstration() {
    // This test explicitly demonstrates the dual assertion strategy:
    // 1. Commands sent match expected variants
    // 2. Final Model state reflects expected outcome

    let model = create_test_model("demo-artifact", false);
    let mut tracker = CommandTracker::new();

    // === ASPECT 1: Command Tracking ===
    // Simulate CheckSerialization command
    let check_effect = Effect::CheckSerialization {
        artifact_index: 0,
        artifact_name: "demo-artifact".to_string(),
        target: "machine-one".to_string(),
        target_type: TargetType::Nixos,
    };

    process_effects_and_track(&mut model.clone(), check_effect, &mut tracker);

    // Assertion 1A: Correct command variant was tracked
    assert_eq!(tracker.commands.len(), 1);
    match &tracker.commands[0] {
        EffectCommand::CheckSerialization { artifact_index, .. } => {
            // Assertion 1B: artifact_index preserved correctly
            assert_eq!(*artifact_index, 0);
        }
        other => panic!("Expected CheckSerialization, got {:?}", other),
    }

    // === ASPECT 2: State Verification ===
    // Apply check result to model
    let (final_model, _) = update(
        model,
        Msg::CheckSerializationResult {
            artifact_index: 0,
            needs_generation: false,
            exists: true,
            result: Ok(()),
            output: None,
        },
    );

    // Assertion 2: Final state matches expected outcome
    assert_eq!(
        final_model.entries[0].status(),
        &ArtifactStatus::UpToDate,
        "Final model state should reflect up-to-date status"
    );
}
