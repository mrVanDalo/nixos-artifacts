use super::super::effect::{Effect, TargetSpec};
use super::super::message::KeyEvent;
use super::super::model::*;
use crossterm::event::KeyCode;

pub(super) fn update_generator_selection(mut model: Model, key: KeyEvent) -> (Model, Effect) {
    let Screen::SelectGenerator(ref mut state) = model.screen else {
        return (model, Effect::None);
    };

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            // Cancel and return to list
            model.screen = Screen::ArtifactList;
            (model, Effect::None)
        }

        KeyCode::Up | KeyCode::Char('k') => {
            if state.selected_index > 0 {
                state.selected_index -= 1;
            }
            (model, Effect::None)
        }

        KeyCode::Down | KeyCode::Char('j') => {
            if state.selected_index + 1 < state.generators.len() {
                state.selected_index += 1;
            }
            (model, Effect::None)
        }

        KeyCode::Enter => {
            // Select the current generator and proceed to prompts/generation
            let selected_path = state
                .generators
                .get(state.selected_index)
                .map(|g| g.path.clone())
                .unwrap_or_default();
            let artifact_index = state.artifact_index;

            // Store the selected generator in the shared entry
            if let Some(ListEntry::Shared(shared)) = model.entries.get_mut(artifact_index) {
                shared.selected_generator = Some(selected_path.clone());

                // Check if prompts are needed
                let prompts: Vec<PromptEntry> = shared
                    .info
                    .prompts
                    .values()
                    .map(|p| PromptEntry {
                        name: p.name.clone(),
                        description: p.description.clone(),
                    })
                    .collect();

                if prompts.is_empty() {
                    // No prompts needed — drop back to the list and dispatch
                    // the generator. Progress is rendered into the right pane
                    // from `ArtifactStatus::Generating`, no screen transition.
                    let run_gen = Effect::RunGenerator {
                        artifact_index,
                        artifact_name: shared.info.artifact_name.clone(),
                        target_spec: TargetSpec::Multi {
                            nixos_targets: shared.info.nixos_targets.clone(),
                            home_targets: shared.info.home_targets.clone(),
                        },
                        prompts: Default::default(),
                    };
                    model.screen = Screen::ArtifactList;
                    model.selected_index = artifact_index;
                    let effect = super::enqueue_or_dispatch(&mut model, run_gen);
                    (model, effect)
                } else {
                    // Need to collect prompts first — switch to inline prompt
                    // input on the artifact list.
                    model.active_prompt = Some(PromptState {
                        artifact_index,
                        artifact_name: shared.info.artifact_name.clone(),
                        description: shared.info.description.clone(),
                        prompts,
                        current_prompt_index: 0,
                        input_mode: InputMode::Line,
                        buffer: String::new(),
                        collected: Default::default(),
                    });
                    model.selected_index = artifact_index;
                    model.screen = Screen::ArtifactList;
                    (model, Effect::None)
                }
            } else {
                // Shouldn't happen, but fall back to list
                model.screen = Screen::ArtifactList;
                (model, Effect::None)
            }
        }

        _ => (model, Effect::None),
    }
}
