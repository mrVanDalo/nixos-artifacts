use super::effect::Effect;
use super::message::{KeyEvent, Msg};
use super::model::*;
use crossterm::event::{KeyCode, KeyModifiers};

/// Compute the initial effect to run when the app starts.
/// This triggers check_serialization for all pending artifacts.
pub fn init(model: &Model) -> Effect {
    let effects: Vec<Effect> = model
        .artifacts
        .iter()
        .enumerate()
        .filter(|(_, e)| e.status == ArtifactStatus::Pending)
        .map(|(i, e)| Effect::CheckSerialization {
            artifact_index: i,
            artifact_name: e.artifact.name.clone(),
            target: e.target.clone(),
            target_type: e.target_type,
        })
        .collect();
    Effect::batch(effects)
}

/// Pure state transition: (Model, Msg) -> (Model, Effect)
/// This function has NO side effects - it only computes new state.
pub fn update(model: Model, msg: Msg) -> (Model, Effect) {
    match (&model.screen, msg) {
        // === Artifact List Screen ===
        (Screen::ArtifactList, Msg::Key(key)) => update_artifact_list(model, key),

        // === Prompt Screen ===
        (Screen::Prompt(_), Msg::Key(key)) => update_prompt(model, key),

        // === Generating Screen ===
        (
            Screen::Generating(_),
            Msg::GeneratorFinished {
                artifact_index,
                result,
            },
        ) => handle_generator_finished(model, artifact_index, result),
        (
            Screen::Generating(_),
            Msg::SerializeFinished {
                artifact_index,
                result,
            },
        ) => handle_serialize_finished(model, artifact_index, result),

        // === Check serialization results (any screen) ===
        (
            _,
            Msg::CheckSerializationResult {
                artifact_index,
                needs_generation,
            },
        ) => handle_check_result(model, artifact_index, needs_generation),

        // === Global ===
        (_, Msg::Quit) => (model, Effect::Quit),
        (_, Msg::Tick) => (model, Effect::None),

        // Unhandled combinations
        _ => (model, Effect::None),
    }
}

fn update_artifact_list(mut model: Model, key: KeyEvent) -> (Model, Effect) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => (model, Effect::Quit),

        KeyCode::Up | KeyCode::Char('k') => {
            if model.selected_index > 0 {
                model.selected_index -= 1;
            }
            (model, Effect::None)
        }

        KeyCode::Down | KeyCode::Char('j') => {
            if model.selected_index + 1 < model.artifacts.len() {
                model.selected_index += 1;
            }
            (model, Effect::None)
        }

        KeyCode::Enter => start_generation_for_selected(model),

        _ => (model, Effect::None),
    }
}

fn start_generation_for_selected(mut model: Model) -> (Model, Effect) {
    let Some(entry) = model.artifacts.get(model.selected_index) else {
        return (model, Effect::None);
    };

    let prompt_state = create_prompt_state(model.selected_index, entry);

    if prompt_state.prompts.is_empty() {
        // No prompts needed, go straight to generating
        let effect = Effect::RunGenerator {
            artifact_index: model.selected_index,
            artifact_name: entry.artifact.name.clone(),
            target: entry.target.clone(),
            target_type: entry.target_type,
            prompts: Default::default(),
        };
        model.screen = Screen::Generating(GeneratingState {
            artifact_index: model.selected_index,
            artifact_name: entry.artifact.name.clone(),
            step: GenerationStep::RunningGenerator,
            log_lines: vec![],
        });
        (model, effect)
    } else {
        model.screen = Screen::Prompt(prompt_state);
        (model, Effect::None)
    }
}

fn update_prompt(mut model: Model, key: KeyEvent) -> (Model, Effect) {
    let Screen::Prompt(ref mut state) = model.screen else {
        return (model, Effect::None);
    };

    match key.code {
        KeyCode::Esc => {
            // Cancel, go back to list
            model.screen = Screen::ArtifactList;
            (model, Effect::None)
        }

        KeyCode::Tab if state.buffer.is_empty() => {
            state.input_mode = state.input_mode.next();
            (model, Effect::None)
        }

        KeyCode::Enter => handle_prompt_enter(model),

        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            handle_prompt_ctrl_d(model)
        }

        KeyCode::Char(c)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            let Screen::Prompt(ref mut state) = model.screen else {
                return (model, Effect::None);
            };
            state.buffer.push(c);
            (model, Effect::None)
        }

        KeyCode::Backspace => {
            let Screen::Prompt(ref mut state) = model.screen else {
                return (model, Effect::None);
            };
            state.buffer.pop();
            (model, Effect::None)
        }

        _ => (model, Effect::None),
    }
}

fn handle_prompt_enter(mut model: Model) -> (Model, Effect) {
    let Screen::Prompt(ref mut state) = model.screen else {
        return (model, Effect::None);
    };

    match state.input_mode {
        InputMode::Line | InputMode::Hidden => {
            // Submit current prompt
            let name = state.prompts[state.current_prompt_index].name.clone();
            state
                .collected
                .insert(name, std::mem::take(&mut state.buffer));
            state.current_prompt_index += 1;

            if state.is_complete() {
                finish_prompts_and_start_generation(model)
            } else {
                // Reset for next prompt
                let Screen::Prompt(ref mut state) = model.screen else {
                    return (model, Effect::None);
                };
                state.input_mode = InputMode::Line;
                (model, Effect::None)
            }
        }
        InputMode::Multiline => {
            state.buffer.push('\n');
            (model, Effect::None)
        }
    }
}

fn handle_prompt_ctrl_d(mut model: Model) -> (Model, Effect) {
    let Screen::Prompt(ref mut state) = model.screen else {
        return (model, Effect::None);
    };

    if state.input_mode == InputMode::Multiline {
        // Submit multiline input
        let name = state.prompts[state.current_prompt_index].name.clone();
        state
            .collected
            .insert(name, std::mem::take(&mut state.buffer));
        state.current_prompt_index += 1;

        if state.is_complete() {
            finish_prompts_and_start_generation(model)
        } else {
            state.input_mode = InputMode::Line;
            (model, Effect::None)
        }
    } else {
        (model, Effect::None)
    }
}

fn finish_prompts_and_start_generation(mut model: Model) -> (Model, Effect) {
    let Screen::Prompt(state) = &model.screen else {
        return (model, Effect::None);
    };

    let artifact_index = state.artifact_index;
    let prompts = state.collected.clone();
    let artifact_name = state.artifact_name.clone();
    let entry = &model.artifacts[artifact_index];
    let target = entry.target.clone();
    let target_type = entry.target_type;

    model.screen = Screen::Generating(GeneratingState {
        artifact_index,
        artifact_name: artifact_name.clone(),
        step: GenerationStep::RunningGenerator,
        log_lines: vec![],
    });

    let effect = Effect::RunGenerator {
        artifact_index,
        artifact_name,
        target,
        target_type,
        prompts,
    };

    (model, effect)
}

fn create_prompt_state(artifact_index: usize, entry: &ArtifactEntry) -> PromptState {
    PromptState {
        artifact_index,
        artifact_name: entry.artifact.name.clone(),
        prompts: entry
            .artifact
            .prompts
            .values()
            .map(|p| PromptEntry {
                name: p.name.clone(),
                description: p.description.clone(),
            })
            .collect(),
        current_prompt_index: 0,
        input_mode: InputMode::Line,
        buffer: String::new(),
        collected: Default::default(),
    }
}

fn handle_check_result(
    mut model: Model,
    artifact_index: usize,
    needs_generation: bool,
) -> (Model, Effect) {
    if let Some(entry) = model.artifacts.get_mut(artifact_index) {
        entry.status = if needs_generation {
            ArtifactStatus::NeedsGeneration
        } else {
            ArtifactStatus::UpToDate
        };
    }
    (model, Effect::None)
}

fn handle_generator_finished(
    mut model: Model,
    artifact_index: usize,
    result: Result<(), String>,
) -> (Model, Effect) {
    match result {
        Ok(()) => {
            // Move to serialization
            if let Screen::Generating(ref mut state) = model.screen {
                state.step = GenerationStep::Serializing;
                state
                    .log_lines
                    .push("Generator completed successfully".into());
            }
            let entry = &model.artifacts[artifact_index];
            let effect = Effect::Serialize {
                artifact_index,
                artifact_name: entry.artifact.name.clone(),
                target: entry.target.clone(),
                target_type: entry.target_type,
                out_dir: Default::default(), // Would come from generator result
            };
            (model, effect)
        }
        Err(e) => {
            if let Some(entry) = model.artifacts.get_mut(artifact_index) {
                entry.status = ArtifactStatus::Failed(e);
            }
            model.screen = Screen::ArtifactList;
            (model, Effect::None)
        }
    }
}

fn handle_serialize_finished(
    mut model: Model,
    artifact_index: usize,
    result: Result<(), String>,
) -> (Model, Effect) {
    match result {
        Ok(()) => {
            if let Some(entry) = model.artifacts.get_mut(artifact_index) {
                entry.status = ArtifactStatus::UpToDate;
            }
        }
        Err(e) => {
            if let Some(entry) = model.artifacts.get_mut(artifact_index) {
                entry.status = ArtifactStatus::Failed(e);
            }
        }
    }
    model.screen = Screen::ArtifactList;
    (model, Effect::None)
}

#[cfg(test)]
mod tests {
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
        Model {
            screen: Screen::ArtifactList,
            artifacts: vec![
                ArtifactEntry {
                    target: "machine-one".to_string(),
                    target_type: TargetType::Nixos,
                    artifact: make_test_artifact("ssh-key", vec!["passphrase"]),
                    status: ArtifactStatus::Pending,
                },
                ArtifactEntry {
                    target: "machine-two".to_string(),
                    target_type: TargetType::Nixos,
                    artifact: make_test_artifact("api-token", vec![]),
                    status: ArtifactStatus::Pending,
                },
            ],
            selected_index: 0,
            error: None,
        }
    }

    #[test]
    fn test_navigate_down() {
        let model = make_test_model();
        let (new_model, effect) = update(model, Msg::Key(KeyEvent::char('j')));

        assert_eq!(new_model.selected_index, 1);
        assert!(effect.is_none());
    }

    #[test]
    fn test_navigate_up() {
        let mut model = make_test_model();
        model.selected_index = 1;
        let (new_model, effect) = update(model, Msg::Key(KeyEvent::char('k')));

        assert_eq!(new_model.selected_index, 0);
        assert!(effect.is_none());
    }

    #[test]
    fn test_navigate_up_at_top_stays() {
        let model = make_test_model();
        let (new_model, _) = update(model, Msg::Key(KeyEvent::char('k')));

        assert_eq!(new_model.selected_index, 0);
    }

    #[test]
    fn test_navigate_down_at_bottom_stays() {
        let mut model = make_test_model();
        model.selected_index = 1;
        let (new_model, _) = update(model, Msg::Key(KeyEvent::char('j')));

        assert_eq!(new_model.selected_index, 1);
    }

    #[test]
    fn test_quit_with_q() {
        let model = make_test_model();
        let (_, effect) = update(model, Msg::Key(KeyEvent::char('q')));

        assert!(effect.is_quit());
    }

    #[test]
    fn test_quit_with_esc() {
        let model = make_test_model();
        let (_, effect) = update(model, Msg::Key(KeyEvent::esc()));

        assert!(effect.is_quit());
    }

    #[test]
    fn test_enter_opens_prompt_screen() {
        let model = make_test_model();
        let (new_model, _) = update(model, Msg::Key(KeyEvent::enter()));

        assert!(matches!(new_model.screen, Screen::Prompt(_)));
    }

    #[test]
    fn test_enter_skips_prompt_if_no_prompts() {
        let mut model = make_test_model();
        model.selected_index = 1; // api-token has no prompts
        let (new_model, effect) = update(model, Msg::Key(KeyEvent::enter()));

        assert!(matches!(new_model.screen, Screen::Generating(_)));
        assert!(matches!(effect, Effect::RunGenerator { .. }));
    }

    #[test]
    fn test_prompt_typing() {
        let mut model = make_test_model();
        model.screen = Screen::Prompt(PromptState {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            prompts: vec![PromptEntry {
                name: "pass".to_string(),
                description: None,
            }],
            current_prompt_index: 0,
            input_mode: InputMode::Line,
            buffer: String::new(),
            collected: Default::default(),
        });

        let (model, _) = update(model, Msg::Key(KeyEvent::char('h')));
        let (model, _) = update(model, Msg::Key(KeyEvent::char('i')));

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
            prompts: vec![PromptEntry {
                name: "pass".to_string(),
                description: None,
            }],
            current_prompt_index: 0,
            input_mode: InputMode::Line,
            buffer: "hello".to_string(),
            collected: Default::default(),
        });

        let (model, _) = update(model, Msg::Key(KeyEvent::backspace()));

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
            prompts: vec![PromptEntry {
                name: "pass".to_string(),
                description: None,
            }],
            current_prompt_index: 0,
            input_mode: InputMode::Line,
            buffer: String::new(),
            collected: Default::default(),
        });

        let (model, _) = update(model, Msg::Key(KeyEvent::tab()));

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
            prompts: vec![PromptEntry {
                name: "pass".to_string(),
                description: None,
            }],
            current_prompt_index: 0,
            input_mode: InputMode::Line,
            buffer: "some text".to_string(),
            collected: Default::default(),
        });

        let (model, _) = update(model, Msg::Key(KeyEvent::tab()));

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
            prompts: vec![],
            current_prompt_index: 0,
            input_mode: InputMode::Line,
            buffer: String::new(),
            collected: Default::default(),
        });

        let (model, _) = update(model, Msg::Key(KeyEvent::esc()));

        assert!(matches!(model.screen, Screen::ArtifactList));
    }
}
