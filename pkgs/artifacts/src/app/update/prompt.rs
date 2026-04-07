use super::super::effect::{Effect, TargetSpec};
use super::super::message::KeyEvent;
use super::super::model::*;
use crossterm::event::{KeyCode, KeyModifiers};

pub(super) fn update_prompt(mut model: Model, key: KeyEvent) -> (Model, Effect) {
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

    model.screen = Screen::Generating(GeneratingState {
        artifact_index,
        artifact_name: artifact_name.clone(),
        step: GenerationStep::RunningGenerator,
        log_lines: vec![],
        exists: false, // Prompt screen means new artifact (no confirmation shown)
    });

    // Build effect based on entry type (using unified TargetSpec)
    let effect = match &model.entries[artifact_index] {
        ListEntry::Single(single) => Effect::RunGenerator {
            artifact_index,
            artifact_name,
            target_spec: TargetSpec::Single(single.target_type.clone()),
            prompts,
        },
        ListEntry::Shared(shared) => Effect::RunGenerator {
            artifact_index,
            artifact_name,
            target_spec: TargetSpec::Multi {
                nixos_targets: shared.info.nixos_targets.clone(),
                home_targets: shared.info.home_targets.clone(),
            },
            prompts,
        },
    };

    (model, effect)
}
