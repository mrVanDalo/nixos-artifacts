use super::super::effect::Effect;
use super::super::message::KeyEvent;
use super::super::model::*;
use crossterm::event::KeyCode;

pub(super) fn update_chronological_log(mut model: Model, key: KeyEvent) -> (Model, Effect) {
    let Screen::ChronologicalLog(ref mut state) = model.screen else {
        return (model, Effect::None);
    };

    let runs_len = model
        .entries
        .get(state.artifact_index)
        .map(|e| e.runs().len())
        .unwrap_or(0);

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            model.screen = Screen::ArtifactList;
            (model, Effect::None)
        }

        KeyCode::Char(' ') | KeyCode::Enter => {
            state.toggle_focused();
            (model, Effect::None)
        }

        KeyCode::Char('+' | '=' | 'e') => {
            state.expand_all(runs_len);
            (model, Effect::None)
        }

        KeyCode::Char('-' | 'c') => {
            state.collapse_all();
            (model, Effect::None)
        }

        KeyCode::Up | KeyCode::Char('k') => {
            state.focus_previous(runs_len);
            (model, Effect::None)
        }

        KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
            state.focus_next(runs_len);
            (model, Effect::None)
        }

        KeyCode::PageUp => {
            if let Some(runs) = model.entries.get(state.artifact_index).map(|e| e.runs()) {
                state.scroll_up(10);
                let max_scroll = state.max_scroll(runs);
                state.clamp_scroll(max_scroll);
            }
            (model, Effect::None)
        }

        KeyCode::PageDown => {
            if let Some(runs) = model.entries.get(state.artifact_index).map(|e| e.runs()) {
                state.scroll_down(10);
                let max_scroll = state.max_scroll(runs);
                state.clamp_scroll(max_scroll);
            }
            (model, Effect::None)
        }

        _ => (model, Effect::None),
    }
}
