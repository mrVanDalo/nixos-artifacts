use super::super::effect::Effect;
use super::super::message::KeyEvent;
use super::super::model::*;
use crossterm::event::KeyCode;

pub(super) fn update_confirm_regenerate(mut model: Model, key: KeyEvent) -> (Model, Effect) {
    let Screen::ConfirmRegenerate(ref mut state) = model.screen else {
        return (model, Effect::None);
    };

    match key.code {
        KeyCode::Left | KeyCode::Char('h') => {
            // Move selection to Leave
            let mut new_state = state.clone();
            new_state.leave_selected = true;
            model.screen = Screen::ConfirmRegenerate(new_state);
            (model, Effect::None)
        }
        KeyCode::Right | KeyCode::Char('l') => {
            // Move selection to Regenerate
            let mut new_state = state.clone();
            new_state.leave_selected = false;
            model.screen = Screen::ConfirmRegenerate(new_state);
            (model, Effect::None)
        }
        KeyCode::Tab => {
            // Toggle selection
            let mut new_state = state.clone();
            new_state.leave_selected = !state.leave_selected;
            model.screen = Screen::ConfirmRegenerate(new_state);
            (model, Effect::None)
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let artifact_index = state.artifact_index;
            if state.leave_selected {
                // Cancel - return to list
                model.screen = Screen::ArtifactList;
                (model, Effect::None)
            } else {
                // Proceed with regeneration
                super::start_generation_for_selected_internal(model, artifact_index)
            }
        }
        KeyCode::Esc => {
            // Cancel - same as Leave
            model.screen = Screen::ArtifactList;
            (model, Effect::None)
        }
        _ => (model, Effect::None),
    }
}
