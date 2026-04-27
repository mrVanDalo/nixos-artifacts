use super::super::effect::{Effect, TargetSpec};
use super::super::message::KeyEvent;
use super::super::model::*;
use crossterm::event::{KeyCode, KeyModifiers};

/// Handle key events while [`Model::active_prompt`] is set.
///
/// Routed unconditionally by [`super::update`] when `active_prompt.is_some()`
/// — the inline prompt swaps the right pane on the artifact list, so neither
/// `selected_index` navigation nor screen-specific keybinds run while a prompt
/// is open.
pub(super) fn update_prompt(mut model: Model, key: KeyEvent) -> (Model, Effect) {
    let Some(state) = model.active_prompt.as_mut() else {
        return (model, Effect::None);
    };

    match key.code {
        KeyCode::Esc => {
            // Cancel: drop the active prompt entirely. Skip semantics for the
            // 'a' flow live in nixos-artifacts-s1f.
            model.active_prompt = None;
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
            state.buffer.push(c);
            (model, Effect::None)
        }

        KeyCode::Backspace => {
            state.buffer.pop();
            (model, Effect::None)
        }

        _ => (model, Effect::None),
    }
}

fn handle_prompt_enter(mut model: Model) -> (Model, Effect) {
    let Some(state) = model.active_prompt.as_mut() else {
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
                finish_prompts_and_dispatch(model)
            } else {
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
    let Some(state) = model.active_prompt.as_mut() else {
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
            finish_prompts_and_dispatch(model)
        } else {
            state.input_mode = InputMode::Line;
            (model, Effect::None)
        }
    } else {
        (model, Effect::None)
    }
}

/// All prompts collected: dispatch the generator, drop this entry from the
/// `a`-flow queue (no-op when the prompt was opened by single-Enter), and
/// either pull up the next queued prompt-bearing artifact or clear the active
/// prompt back to plain log view.
fn finish_prompts_and_dispatch(mut model: Model) -> (Model, Effect) {
    let Some(state) = model.active_prompt.take() else {
        return (model, Effect::None);
    };

    let artifact_index = state.artifact_index;
    let prompts = state.collected;
    let artifact_name = state.artifact_name;

    // Remove from the `a`-flow queue — single-Enter never enqueued, so this
    // is a no-op for that path.
    model.generate_queue.remove(&artifact_index);

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

    // Advance to the next queued prompt-bearing entry. When nothing remains
    // (single-Enter case, or the `a` flow finished its prompt-bearing tail)
    // `active_prompt` stays `None`, and the right pane reverts to logs.
    super::set_next_active_prompt(&mut model);

    (model, effect)
}
