use super::super::effect::Effect;
use super::super::message::KeyEvent;
use super::super::model::*;
use super::build_run_generator_effect_for;
use crossterm::event::KeyCode;

pub(super) fn update_artifact_list(mut model: Model, key: KeyEvent) -> (Model, Effect) {
    match key.code {
        // Esc no longer quits — it is the trigger for the universal Esc-Esc
        // cancel chord (see `super::update`). `q` remains the explicit quit.
        KeyCode::Char('q') => (model, Effect::Quit),

        KeyCode::Up | KeyCode::Char('k') => {
            if model.selected_index > 0 {
                model.selected_index -= 1;
            }
            (model, Effect::None)
        }

        KeyCode::Down | KeyCode::Char('j') => {
            if model.selected_index + 1 < model.entries.len() {
                model.selected_index += 1;
            }
            (model, Effect::None)
        }

        KeyCode::Tab => {
            model.selected_log_step = model.selected_log_step.next();
            (model, Effect::None)
        }

        KeyCode::Enter => start_generation_for_selected(model),

        KeyCode::Char('a') => generate_all(model),

        KeyCode::Char('l') => open_chronological_log_view(model),

        _ => (model, Effect::None),
    }
}

/// Walks the artifact list and partitions entries by status:
///
/// - `UpToDate` / `Generating` / `Failed`: skipped (no regen confirmation; no
///   retry of failed entries from this flow).
/// - `Pending`: enqueued in `model.generate_queue`. The check is already in
///   flight (started by [`super::init::init`]); when it resolves, the queue
///   drain in [`super::handle_check_result`] picks it up.
/// - `NeedsGeneration`: dispatched as `Effect::RunGenerator` immediately when
///   the entry has no prompts and (for shared) exactly one generator. Entries
///   that need user input are queued and surfaced via the inline-prompt
///   right-pane (see `set_next_active_prompt` in `super::mod`).
fn generate_all(mut model: Model) -> (Model, Effect) {
    let mut effects: Vec<Effect> = Vec::new();

    for index in 0..model.entries.len() {
        let entry = &model.entries[index];
        match entry.status() {
            ArtifactStatus::UpToDate
            | ArtifactStatus::Generating(_)
            | ArtifactStatus::Failed { .. }
            | ArtifactStatus::Cancelled { .. } => {}
            ArtifactStatus::Pending => {
                model.generate_queue.insert(index);
            }
            ArtifactStatus::NeedsGeneration => match build_run_generator_effect_for(entry, index) {
                Some(effect) => effects.push(effect),
                None => {
                    model.generate_queue.insert(index);
                }
            },
        }
    }

    // Surface the first queued prompt-bearing entry inline (right pane). If
    // none are ready (queue empty, or only Pending entries waiting on their
    // check) `active_prompt` stays as it was — typically `None`.
    if model.active_prompt.is_none() {
        super::set_next_active_prompt(&mut model);
    }

    (model, Effect::batch(effects))
}

fn start_generation_for_selected(mut model: Model) -> (Model, Effect) {
    // First, check if we need to show confirmation dialog
    let should_show_dialog = {
        let Some(entry) = model.entries.get(model.selected_index) else {
            return (model, Effect::None);
        };

        // Show confirmation dialog if artifact exists (UpToDate status)
        // User is trying to regenerate an existing artifact
        if matches!(entry.status(), ArtifactStatus::UpToDate) {
            // Extract info needed for the dialog
            let artifact_name = entry.artifact_name().to_string();
            let affected_targets = match entry {
                ListEntry::Single(single) => vec![single.target_type.target_name().to_string()],
                ListEntry::Shared(shared) => {
                    let mut targets: Vec<String> = shared
                        .info
                        .nixos_targets
                        .iter()
                        .map(|t| format!("nixos: {}", t))
                        .chain(
                            shared
                                .info
                                .home_targets
                                .iter()
                                .map(|t| format!("home: {}", t)),
                        )
                        .collect();
                    if targets.len() > 5 {
                        targets.truncate(5);
                        targets.push("...".to_string());
                    }
                    targets
                }
            };
            Some((artifact_name, affected_targets))
        } else {
            None
        }
    };

    // Show dialog if needed
    if let Some((artifact_name, affected_targets)) = should_show_dialog {
        model.screen = Screen::ConfirmRegenerate(ConfirmRegenerateState {
            artifact_index: model.selected_index,
            artifact_name,
            affected_targets,
            leave_selected: true, // Safe default
        });
        return (model, Effect::None);
    }

    // For new artifacts or UpToDate ones, proceed directly
    let artifact_index = model.selected_index;
    super::start_generation_for_selected_internal(model, artifact_index)
}

fn open_chronological_log_view(mut model: Model) -> (Model, Effect) {
    let artifact_index = model.selected_index;

    if let Some(entry) = model.entries.get(artifact_index) {
        let state = ChronologicalLogState::new(
            artifact_index,
            entry.artifact_name().to_string(),
            entry.runs().len(),
        );
        model.screen = Screen::ChronologicalLog(state);
    }

    (model, Effect::None)
}
