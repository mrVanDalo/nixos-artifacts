//! Pure state transition functions for the Elm Architecture.
//!
//! This module implements the `update` function - the core of the Elm Architecture.
//! It takes the current model and a message, and returns a new model and an effect.
//! All functions in this module are pure (no side effects), making them easy to test.
//!
//! # Structure
//!
//! Each screen has its own submodule with an update handler:
//! - [`artifact_list`]: List navigation and generation triggers
//! - [`generator_selection`]: Generator selection for shared artifacts
//! - [`confirm_regenerate`]: Regeneration confirmation dialog
//! - [`prompt`]: Prompt input and submission
//! - [`generating`]: Generation/serialization result handlers
//! - [`chronological_log`]: Log viewing and navigation
//!
//! Cross-screen handlers (check results, output streaming, quit, tick)
//! remain in this dispatcher module.

mod artifact_list;
mod chronological_log;
mod confirm_regenerate;
mod generating;
mod generator_selection;
mod init;
mod prompt;

pub use init::init;

use std::collections::HashMap;
use std::fmt::Write;
use std::time::{Duration, Instant};

use crossterm::event::KeyCode;

use super::effect::{Effect, TargetSpec};
use super::message::{Message, ScriptOutput};
use super::model::*;

/// Window during which a second Esc keypress is interpreted as the cancel
/// chord. Picked to match common UI chord conventions; tunable later if user
/// feedback shows it's too tight or too loose.
const ESC_CHORD_WINDOW: Duration = Duration::from_millis(500);

// Re-exported for use by tests via `super::*`
#[cfg(test)]
use super::message::KeyEvent;

/// Soft-cancel the in-flight 'a' (generate-all) batch.
///
/// This is the foundation other beads (Esc-Esc chord, Ctrl-C semantics fix)
/// invoke from their key handlers. The function makes only model-side
/// changes and returns the [`Effect::CancelQueue`] descriptor; the runtime
/// turns that descriptor into a signal on the dedicated cancel channel,
/// which drains the background FIFO of every effect that has not started
/// executing yet. The currently-running effect, if any, runs to natural
/// completion.
///
/// Concretely, this:
/// * reverts every entry whose status is [`ArtifactStatus::Pending`] to
///   [`ArtifactStatus::NeedsGeneration`]. After cancel we don't know whether
///   their queued check would have resolved up-to-date or stale, and the
///   yellow `!` is the safer default — the user can re-trigger generation
///   from a known state instead of staring at a frozen `○`.
/// * clears [`Model::generate_queue`] so future check results no longer
///   auto-dispatch generators or pull entries into the inline-prompt flow.
/// * clears [`Model::active_prompt`] so the right pane reverts to logs.
/// * forces [`Screen::ArtifactList`]; any modal that was open (prompt
///   confirmation, generator selection, log view) is dismissed.
///
/// Calling this when nothing is queued is a deliberate no-op except for the
/// `Effect::CancelQueue` return value, which still routes to the background
/// and drains an already-empty FIFO. No new
/// [`crate::app::model::ArtifactStatus`] variant is introduced — entries
/// that are mid-`Generating` are left alone and resolve naturally.
pub fn cancel_queue(mut model: Model) -> (Model, Effect) {
    for entry in &mut model.entries {
        if matches!(entry.status(), ArtifactStatus::Pending) {
            *entry.status_mut() = ArtifactStatus::NeedsGeneration;
        }
    }
    model.generate_queue.clear();
    model.pipeline_queue.clear();
    model.in_flight = None;
    model.active_prompt = None;
    model.screen = Screen::ArtifactList;
    (model, Effect::CancelQueue)
}

/// Pure state transition: (Model, Message) -> (Model, Effect)
/// This function has NO side effects - it only computes new state.
pub fn update(model: Model, msg: Message) -> (Model, Effect) {
    // Esc-Esc cancel chord (universal): a second Esc within
    // [`ESC_CHORD_WINDOW`] of the first triggers `cancel_queue`, regardless
    // of which screen the user is on. The first Esc records the timestamp
    // and falls through to the screen-specific handler so its existing
    // single-Esc behavior still runs. Any non-Esc key clears the timer so
    // intervening input breaks the sequence; non-key messages (Tick, async
    // results) are pass-through.
    let model = match &msg {
        Message::Key(key) if matches!(key.code, KeyCode::Esc) => {
            let now = Instant::now();
            if let Some(prev) = model.last_esc_at
                && now.duration_since(prev) < ESC_CHORD_WINDOW
            {
                let mut model = model;
                model.last_esc_at = None;
                return cancel_queue(model);
            }
            let mut model = model;
            model.last_esc_at = Some(now);
            model
        }
        Message::Key(_) => {
            let mut model = model;
            model.last_esc_at = None;
            model
        }
        _ => model,
    };

    // Inline prompt input takes over keys whenever it is active and the user
    // is on the artifact list. Other screens (Generating, ChronologicalLog,
    // dialogs) keep their own key handling so the prompt is held but inert
    // until the user returns to the list.
    if model.active_prompt.is_some()
        && matches!(model.screen, Screen::ArtifactList)
        && let Message::Key(ref key) = msg
    {
        return prompt::update_prompt(model, key.clone());
    }

    match (&model.screen, msg) {
        // === Artifact List Screen ===
        (Screen::ArtifactList, Message::Key(key)) => {
            artifact_list::update_artifact_list(model, key)
        }

        // === Generator Selection Screen ===
        (Screen::SelectGenerator(_), Message::Key(key)) => {
            generator_selection::update_generator_selection(model, key)
        }

        // === Confirm Regenerate Screen ===
        (Screen::ConfirmRegenerate(_), Message::Key(key)) => {
            confirm_regenerate::update_confirm_regenerate(model, key)
        }

        // === Generator/Serialize results (any screen) ===
        // Results may arrive while the user is on the artifact list or any
        // other screen, so we accept them unconditionally. Generation progress
        // renders into the right pane via the selected entry's status, so
        // there is no separate screen to leave on completion.
        (
            _,
            Message::GeneratorFinished {
                artifact_index,
                result,
            },
        ) => generating::handle_generator_finished(model, artifact_index, result),
        (_, Message::GeneratorCancelled { artifact_index }) => {
            generating::handle_generator_cancelled(model, artifact_index)
        }
        (
            _,
            Message::SerializeFinished {
                artifact_index,
                result,
            },
        ) => generating::handle_serialize_finished(model, artifact_index, result),

        // === Chronological Log Screen ===
        (Screen::ChronologicalLog(_), Message::Key(key)) => {
            chronological_log::update_chronological_log(model, key)
        }

        // === Check serialization results (any screen, single or shared) ===
        (
            _,
            Message::CheckSerializationResult {
                artifact_index,
                status,
                result,
            },
        ) => handle_check_result(model, artifact_index, status, result),

        // === Streaming output (any screen) ===
        (
            _,
            Message::OutputLine {
                artifact_index,
                stream,
                content,
            },
        ) => handle_output_line(model, artifact_index, stream, content),

        // === Global ===
        (_, Message::Quit) => (model, Effect::Quit),
        (_, Message::Tick) => {
            let mut model = model;
            model.tick_count += 1;
            (model, Effect::None)
        }

        // Unhandled combinations
        _ => (model, Effect::None),
    }
}

pub(crate) fn create_prompt_state(artifact_index: usize, entry: &ArtifactEntry) -> PromptState {
    PromptState {
        artifact_index,
        artifact_name: entry.artifact.name.clone(),
        description: entry.artifact.description.clone(),
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
    status: ArtifactStatus,
    result: Result<ScriptOutput, String>,
) -> (Model, Effect) {
    {
        let Some(entry) = model.entries.get_mut(artifact_index) else {
            return (model, Effect::None);
        };
        match result {
            Ok(output) => {
                // Add captured script output to logs using helper methods
                if !output.stdout_lines.is_empty() || !output.stderr_lines.is_empty() {
                    entry
                        .step_logs_mut()
                        .append_stdout(Step::Check, &output.stdout_lines);
                    entry
                        .step_logs_mut()
                        .append_stderr(Step::Check, &output.stderr_lines);
                }

                // Set status from check result
                *entry.status_mut() = status.clone();

                // Add status summary log entry
                let (level, message) = match status {
                    ArtifactStatus::NeedsGeneration => {
                        (LogLevel::Info, "Artifact needs regeneration".to_string())
                    }
                    ArtifactStatus::UpToDate => {
                        (LogLevel::Success, "Already up to date".to_string())
                    }
                    _ => (LogLevel::Info, "Unknown status".to_string()),
                };
                entry
                    .step_logs_mut()
                    .check
                    .push(LogEntry { level, message });
            }
            Err(e) => {
                let artifact_error = ArtifactError::IoError { context: e.clone() };
                entry.step_logs_mut().check.push(LogEntry {
                    level: LogLevel::Error,
                    message: e,
                });
                *entry.status_mut() = ArtifactStatus::Failed {
                    error: artifact_error,
                    output: String::new(),
                };
            }
        }
    }

    // Drain the generate-all queue. Pending entries land here when the user
    // hits `a`; once their check resolves we either dispatch a generator
    // (NeedsGeneration, no prompts) or drop the entry from the queue
    // (UpToDate, or anything else terminal). NeedsGeneration entries that need
    // user input stay queued for the inline-prompt flow to drain.
    if model.generate_queue.contains(&artifact_index) {
        let Some(entry) = model.entries.get(artifact_index) else {
            return (model, Effect::None);
        };
        match entry.status() {
            ArtifactStatus::NeedsGeneration => {
                if let Some(effect) = build_run_generator_effect_for(entry, artifact_index) {
                    model.generate_queue.remove(&artifact_index);
                    // Push onto the gen→ser pipeline so a check resolving
                    // mid-batch doesn't bypass the in-flight artifact. See
                    // nixos-artifacts-tje.
                    let dispatched = enqueue_or_dispatch(&mut model, effect);
                    return (model, dispatched);
                }
                // Prompt-bearing: keep queued, surface the inline prompt if
                // nothing else is currently being collected.
                if model.active_prompt.is_none() {
                    set_next_active_prompt(&mut model);
                }
            }
            ArtifactStatus::UpToDate
            | ArtifactStatus::Failed { .. }
            | ArtifactStatus::Cancelled { .. } => {
                model.generate_queue.remove(&artifact_index);
            }
            _ => {}
        }
    }

    (model, Effect::None)
}

/// Pick the next queued prompt-bearing entry (lowest index wins for stable
/// ordering) and seed `model.active_prompt` from it. Clears `active_prompt`
/// when no queued entry is ready to collect prompts.
///
/// Called after every prompt submission and after a queued check resolves to
/// `NeedsGeneration` with prompts. The single-Enter path bypasses this — it
/// sets `active_prompt` directly without touching `generate_queue`.
pub(crate) fn set_next_active_prompt(model: &mut Model) {
    let mut candidates: Vec<usize> = model.generate_queue.iter().copied().collect();
    candidates.sort_unstable();

    for index in candidates {
        let Some(entry) = model.entries.get(index) else {
            continue;
        };
        if !matches!(entry.status(), ArtifactStatus::NeedsGeneration) {
            continue;
        }
        if let Some(state) = build_prompt_state_for(entry, index) {
            model.active_prompt = Some(state);
            // Lock selection to the prompt's artifact so the right pane
            // renders the prompt against a stable target. The dispatcher
            // already routes j/k to the prompt handler while a prompt is
            // open, so this only needs to be set at assignment time.
            model.selected_index = index;
            return;
        }
    }

    model.active_prompt = None;
}

/// Build a [`PromptState`] from a list entry. Returns `None` for entries that
/// don't need prompts or (for shared) need generator selection first.
pub(crate) fn build_prompt_state_for(
    entry: &ListEntry,
    artifact_index: usize,
) -> Option<PromptState> {
    match entry {
        ListEntry::Single(single) => {
            if single.artifact.prompts.is_empty() {
                return None;
            }
            Some(PromptState {
                artifact_index,
                artifact_name: single.artifact.name.clone(),
                description: single.artifact.description.clone(),
                prompts: single
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
            })
        }
        ListEntry::Shared(shared) => {
            // Generator must be resolved before prompts; an entry waiting for
            // a generator-selection dialog isn't a prompt candidate yet.
            if shared.info.prompts.is_empty()
                || shared.info.generators.len() != 1
                || shared.info.error.is_some()
            {
                return None;
            }
            Some(PromptState {
                artifact_index,
                artifact_name: shared.info.artifact_name.clone(),
                description: shared.info.description.clone(),
                prompts: shared
                    .info
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
            })
        }
    }
}

/// Push a `RunGenerator` to the pipeline and return either that effect
/// (if the pipeline is empty and we can dispatch immediately) or
/// `Effect::None` (if a previous artifact is still in flight). Other effect
/// variants pass through unchanged. The pipeline is drained one slot at a
/// time so the user sees gen0→ser0→gen1→ser1 rather than all-gens-then-all-
/// sers. See nixos-artifacts-tje.
pub(super) fn enqueue_or_dispatch(model: &mut Model, effect: Effect) -> Effect {
    match effect {
        Effect::RunGenerator { .. } => {
            model.pipeline_queue.push_back(effect);
            pump_pipeline(model)
        }
        other => other,
    }
}

/// If nothing is currently in flight, pop the next `RunGenerator` from the
/// pipeline queue, mark it in flight, and return it for dispatch. Otherwise
/// return `Effect::None`.
///
/// Dispatch flips the artifact's status to
/// [`ArtifactStatus::Generating`] with the `Generate` substep so the right
/// pane in `Screen::ArtifactList` switches from logs to live progress.
pub(super) fn pump_pipeline(model: &mut Model) -> Effect {
    if model.in_flight.is_some() {
        return Effect::None;
    }
    let Some(effect) = model.pipeline_queue.pop_front() else {
        return Effect::None;
    };
    if let Effect::RunGenerator { artifact_index, .. } = &effect {
        model.in_flight = Some(*artifact_index);
        if let Some(entry) = model.entries.get_mut(*artifact_index) {
            *entry.status_mut() = ArtifactStatus::Generating(GeneratingSubstate {
                step: Step::Generate,
                output: String::new(),
            });
        }
    }
    effect
}

/// Builds an `Effect::RunGenerator` for an entry that can be dispatched without
/// further user interaction. Returns `None` if the entry has prompts to
/// collect or (for shared artifacts) more than one generator to choose from —
/// those need to flow through the prompt / generator-selection screens.
pub(super) fn build_run_generator_effect_for(
    entry: &ListEntry,
    artifact_index: usize,
) -> Option<Effect> {
    match entry {
        ListEntry::Single(single) => {
            if !single.artifact.prompts.is_empty() {
                return None;
            }
            Some(Effect::RunGenerator {
                artifact_index,
                artifact_name: single.artifact.name.clone(),
                target_spec: TargetSpec::Single(single.target_type.clone()),
                prompts: HashMap::new(),
            })
        }
        ListEntry::Shared(shared) => {
            if !shared.info.prompts.is_empty()
                || shared.info.generators.len() != 1
                || shared.info.error.is_some()
            {
                return None;
            }
            Some(Effect::RunGenerator {
                artifact_index,
                artifact_name: shared.info.artifact_name.clone(),
                target_spec: TargetSpec::Multi {
                    nixos_targets: shared.info.nixos_targets.clone(),
                    home_targets: shared.info.home_targets.clone(),
                },
                prompts: HashMap::new(),
            })
        }
    }
}

// === Helper for generation flow ===

/// Data extracted from a `ListEntry` before the mutable reborrow of `model.entries`.
///
/// For `Single` entries only `exists_before` and `artifact_name` are meaningful;
/// the shared-specific fields stay at their `Default` and are never read.
#[derive(Default)]
struct EntryData {
    exists_before: bool,
    artifact_name: String,
    generator_path: Option<String>,
    description: Option<String>,
    prompts: Vec<crate::config::make::PromptDef>,
    nixos_targets: Vec<String>,
    home_targets: Vec<String>,
    generators: Vec<crate::config::make::GeneratorInfo>,
}

#[allow(clippy::too_many_lines)]
pub(crate) fn start_generation_for_selected_internal(
    mut model: Model,
    artifact_index: usize,
) -> (Model, Effect) {
    // Extract all needed data before any mutable borrow
    let entry_data: Option<EntryData> = {
        let Some(entry) = model.entries.get(artifact_index) else {
            return (model, Effect::None);
        };
        match entry {
            ListEntry::Single(single) => Some(EntryData {
                exists_before: matches!(single.status, ArtifactStatus::UpToDate),
                artifact_name: single.artifact.name.clone(),
                ..Default::default()
            }),
            ListEntry::Shared(shared) => {
                if shared.info.error.is_some() {
                    return (model, Effect::None);
                }
                Some(EntryData {
                    exists_before: matches!(shared.status, ArtifactStatus::UpToDate),
                    artifact_name: shared.info.artifact_name.clone(),
                    generator_path: shared.info.generators.first().map(|g| g.path.clone()),
                    description: shared.info.description.clone(),
                    prompts: shared.info.prompts.values().cloned().collect(),
                    nixos_targets: shared.info.nixos_targets.clone(),
                    home_targets: shared.info.home_targets.clone(),
                    generators: shared.info.generators.clone(),
                })
            }
        }
    };

    let Some(EntryData {
        exists_before,
        artifact_name,
        generator_path,
        description,
        prompts,
        nixos_targets,
        home_targets,
        generators,
    }) = entry_data
    else {
        return (model, Effect::None);
    };

    // Regeneration skips the check step, so seed a fresh run here. First-time
    // generation (NeedsGeneration) continues the run started by init().
    if exists_before && let Some(entry) = model.entries.get_mut(artifact_index) {
        entry.start_new_run();
    }

    // Now we can mutate model without borrowing entry
    let Some(entry) = model.entries.get(artifact_index) else {
        return (model, Effect::None);
    };

    // Default screen for any path that just dispatches a generator effect.
    // Inline-prompt branches and the multi-generator selection branch will
    // override this below.
    model.screen = Screen::ArtifactList;

    match entry {
        ListEntry::Single(single) => {
            let prompt_state = create_prompt_state(artifact_index, single);
            let artifact_name = single.artifact.name.clone();
            let target_type = single.target_type.clone();

            if prompt_state.prompts.is_empty() {
                let run_gen = Effect::RunGenerator {
                    artifact_index,
                    artifact_name: artifact_name.clone(),
                    target_spec: TargetSpec::Single(target_type),
                    prompts: Default::default(),
                };
                let effect = enqueue_or_dispatch(&mut model, run_gen);
                (model, effect)
            } else {
                // Inline prompt: stay on the artifact list and swap the right
                // pane via `active_prompt`. The runtime picks this up in
                // `tui/views/list.rs::render_log_panel`.
                model.active_prompt = Some(prompt_state);
                model.selected_index = artifact_index;
                model.screen = Screen::ArtifactList;
                (model, Effect::None)
            }
        }
        ListEntry::Shared(_) => {
            if generators.len() == 1 {
                if let Some(generator_path) = generator_path
                    && let Some(ListEntry::Shared(shared)) = model.entries.get_mut(artifact_index)
                {
                    shared.selected_generator = Some(generator_path);
                }

                if prompts.is_empty() {
                    let run_gen = Effect::RunGenerator {
                        artifact_index,
                        artifact_name: artifact_name.clone(),
                        target_spec: TargetSpec::Multi {
                            nixos_targets: nixos_targets.clone(),
                            home_targets: home_targets.clone(),
                        },
                        prompts: Default::default(),
                    };
                    let effect = enqueue_or_dispatch(&mut model, run_gen);
                    (model, effect)
                } else {
                    model.active_prompt = Some(PromptState {
                        artifact_index,
                        artifact_name,
                        description,
                        prompts: prompts
                            .iter()
                            .map(|p| PromptEntry {
                                name: p.name.clone(),
                                description: p.description.clone(),
                            })
                            .collect(),
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
                model.screen = Screen::SelectGenerator(SelectGeneratorState {
                    artifact_index,
                    artifact_name: artifact_name.clone(),
                    description,
                    generators,
                    selected_index: 0,
                    prompts,
                    nixos_targets,
                    home_targets,
                });
                (model, Effect::None)
            }
        }
    }
}

/// Handles streaming output line received during script execution.
fn handle_output_line(
    mut model: Model,
    artifact_index: usize,
    stream: OutputStream,
    content: String,
) -> (Model, Effect) {
    let Some(entry) = model.entries.get_mut(artifact_index) else {
        return (model, Effect::None);
    };
    // Determine log level from stream type
    let level = match stream {
        OutputStream::Stdout => LogLevel::Output,
        OutputStream::Stderr => LogLevel::Error,
    };

    // Determine current step based on status
    // Use the model's selected_log_step to determine where to append
    // This ensures streaming output goes to the correct step being viewed
    let step = model.selected_log_step;

    // Append to appropriate step logs
    entry.step_logs_mut().get_mut(step).push(LogEntry {
        level,
        message: content,
    });
    (model, Effect::None)
}

/// Formats accumulated step logs from check and generate phases for error output.
pub(crate) fn format_step_logs(entry: &ListEntry) -> String {
    let mut output = String::new();
    for log in &entry.step_logs().check {
        let _ = writeln!(output, "[check] {}", log.message);
    }
    for log in &entry.step_logs().generate {
        let _ = writeln!(output, "[generate] {}", log.message);
    }
    output
}

#[cfg(test)]
mod tests;
