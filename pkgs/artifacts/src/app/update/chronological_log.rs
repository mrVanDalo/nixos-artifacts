use super::super::effect::Effect;
use super::super::message::KeyEvent;
use super::super::model::*;
use crossterm::event::KeyCode;

pub(super) fn update_chronological_log(mut model: Model, key: KeyEvent) -> (Model, Effect) {
    let Screen::ChronologicalLog(ref mut state) = model.screen else {
        return (model, Effect::None);
    };

    // Get the step logs for scroll calculations
    let step_logs = model
        .entries
        .get(state.artifact_index)
        .map(|e| e.step_logs());

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            // Return to artifact list
            model.screen = Screen::ArtifactList;
            (model, Effect::None)
        }

        KeyCode::Char(' ') | KeyCode::Enter => {
            // Space or Enter toggles the focused section
            if let Some(step) = state.focused_section {
                state.toggle_section(step);
            }
            (model, Effect::None)
        }

        KeyCode::Char('+' | '=') => {
            // '+' key expands all sections
            state.expand_all();
            (model, Effect::None)
        }

        KeyCode::Char('-') => {
            // '-' key collapses all sections
            state.collapse_all();
            (model, Effect::None)
        }

        KeyCode::Char('e') => {
            // 'e' key expands all sections (legacy)
            state.expand_all();
            (model, Effect::None)
        }

        KeyCode::Char('c') => {
            // 'c' key collapses all sections (legacy)
            state.collapse_all();
            (model, Effect::None)
        }

        KeyCode::Up | KeyCode::Char('k') => {
            // Move focus to previous section
            state.focus_previous();
            (model, Effect::None)
        }

        KeyCode::Down | KeyCode::Char('j') => {
            // Move focus to next section
            state.focus_next();
            (model, Effect::None)
        }

        KeyCode::PageUp => {
            // Page up - scroll content
            if let Some(logs) = step_logs {
                state.scroll_up(10);
                let max_scroll = state.max_scroll(logs);
                state.clamp_scroll(max_scroll);
            }
            (model, Effect::None)
        }

        KeyCode::PageDown => {
            // Page down - scroll content
            if let Some(logs) = step_logs {
                state.scroll_down(10);
                let max_scroll = state.max_scroll(logs);
                state.clamp_scroll(max_scroll);
            }
            (model, Effect::None)
        }

        KeyCode::Tab => {
            // Move focus to next section
            state.focus_next();
            (model, Effect::None)
        }

        _ => (model, Effect::None),
    }
}
