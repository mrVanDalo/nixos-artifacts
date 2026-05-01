//! Application state types for the Elm Architecture implementation.
//!
//! This module defines all the state types used by the application:
//! - [`Model`]: The root application state
//! - [`Screen`]: Current view/screen being displayed
//! - [`ArtifactStatus`]: Lifecycle state of artifact generation
//! - [`ListEntry`]: Unified list entry type (single or shared artifacts)
//! - Various screen states for prompts, generation, logs, etc.
//!
//! # State Immutability
//!
//! All types in this module are designed for immutability:
//! - All fields are `pub` for pattern matching
//! - Update functions create new instances rather than mutate
//! - Cloning is cheap (most fields are small or reference-counted)

use std::collections::{HashSet, VecDeque};
use std::time::Instant;

use crate::app::effect::Effect;

use super::artifact::ListEntry;
use super::log::{ChronologicalLogState, Step, Warning};
use super::prompt::PromptState;
use super::screen_state::{ConfirmRegenerateState, DoneState, SelectGeneratorState};

/// Root application state containing all UI data.
///
/// This is the single source of truth for the TUI. All state changes
/// flow through the update function, producing a new Model.
#[derive(Debug, Clone, Default)]
pub struct Model {
    /// Current screen being displayed (determines what view renders)
    pub screen: Screen,
    /// Unified list of entries displayed in the artifact list
    /// Contains both single artifacts and shared artifacts
    pub entries: Vec<ListEntry>,
    /// Currently selected entry index in the artifact list
    pub selected_index: usize,
    /// Currently selected log step for viewing output
    pub selected_log_step: Step,
    /// Critical error message (displayed in a banner)
    pub error: Option<String>,
    /// Non-blocking warnings about backend capability issues
    pub warnings: Vec<Warning>,
    /// Animation frame counter for spinner animation
    pub tick_count: usize,
    /// Indexes of entries enqueued by the `a` (generate-all) flow whose
    /// `check_serialization` is still in flight. When a queued entry's check
    /// resolves to `NeedsGeneration` its generator is dispatched and the index
    /// is removed; an `UpToDate` result drops the index silently.
    pub generate_queue: HashSet<usize>,
    /// Inline prompt collection state. When `Some`, the artifact list view
    /// swaps its right-side log panel for prompt input; key events are routed
    /// to the prompt handler regardless of `selected_index`. Submission
    /// dispatches `Effect::RunGenerator` and either advances to the next
    /// queued prompt-bearing artifact (`a` flow) or clears back to logs
    /// (single-Enter flow).
    pub active_prompt: Option<PromptState>,
    /// Timestamp of the most recent Esc keypress. Powers the universal
    /// Esc-Esc cancel chord: a second Esc within 500ms of this instant
    /// dispatches `cancel_queue` regardless of which screen the user is on.
    /// Set on every Esc, cleared on every non-Esc key, and cleared again
    /// after the chord fires. Non-key messages (Tick, async results) leave
    /// it alone so a long-running tick stream cannot widen the chord window
    /// or close it prematurely.
    pub last_esc_at: Option<Instant>,
    /// `RunGenerator` effects waiting for an open pipeline slot. The 'a'
    /// flow used to batch every generator into the FIFO at once, which made
    /// all generators run before any serialize (each Serialize is only
    /// enqueued after the runtime drains its `GeneratorFinished`). Now every
    /// dispatcher pushes here instead; the pipeline drains one entry at a
    /// time so the user-visible order is gen0→ser0→gen1→ser1→… See
    /// nixos-artifacts-tje.
    pub pipeline_queue: VecDeque<Effect>,
    /// `artifact_index` of the artifact currently mid-pipeline (gen or ser
    /// in flight). `None` between artifacts and at start. Set when a
    /// `RunGenerator` is dispatched; cleared in the serialize/generator
    /// terminal handlers. The pipeline only advances when this is `None`.
    pub in_flight: Option<usize>,
}

/// Current screen/view being displayed in the TUI.
///
/// The screen determines which view is rendered and which update
/// handler processes keyboard input.
///
/// Note: prompt input no longer has its own screen — it is collected inline
/// on the `ArtifactList` view via [`Model::active_prompt`]. See the design
/// in nixos-artifacts-psg.
#[derive(Debug, Clone, Default)]
pub enum Screen {
    /// Main artifact list view - the default screen
    /// Shows all artifacts with their status
    #[default]
    ArtifactList,
    /// Generator selection dialog for shared artifacts with multiple generators
    SelectGenerator(SelectGeneratorState),
    /// Confirmation dialog before regenerating existing artifacts
    ConfirmRegenerate(ConfirmRegenerateState),
    /// Completion screen showing generation summary
    Done(DoneState),
    /// Chronological log view with expandable sections per generation step
    ChronologicalLog(ChronologicalLogState),
}
