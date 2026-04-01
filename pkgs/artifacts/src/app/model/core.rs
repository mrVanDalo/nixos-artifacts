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

use super::artifact::ListEntry;
use super::log::{ChronologicalLogState, LogStep, Warning};
use super::prompt::PromptState;
use super::screen_state::{
    ConfirmRegenerateState, DoneState, GeneratingState, SelectGeneratorState,
};

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
    pub selected_log_step: LogStep,
    /// Critical error message (displayed in a banner)
    pub error: Option<String>,
    /// Non-blocking warnings about backend capability issues
    pub warnings: Vec<Warning>,
    /// Animation frame counter for spinner animation
    pub tick_count: usize,
}

/// Current screen/view being displayed in the TUI.
///
/// The screen determines which view is rendered and which update
/// handler processes keyboard input.
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
    /// Prompt input screen for collecting user input
    Prompt(PromptState),
    /// Generation progress screen with live output
    Generating(GeneratingState),
    /// Completion screen showing generation summary
    Done(DoneState),
    /// Chronological log view with expandable sections per generation step
    ChronologicalLog(ChronologicalLogState),
}
