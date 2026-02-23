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

use crate::config::make::{ArtifactDef, GeneratorInfo, SharedArtifactInfo};
use ratatui::style::{Color, Style};
use std::collections::{HashMap, HashSet};

/// Root application state containing all UI data.
///
/// This is the single source of truth for the TUI. All state changes
/// flow through the update function, producing a new Model.
#[derive(Debug, Clone, Default)]
pub struct Model {
    /// Current screen being displayed (determines what view renders)
    pub screen: Screen,
    /// Legacy field - list of per-target artifacts (kept for backward compatibility)
    pub artifacts: Vec<ArtifactEntry>,
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

/// A per-target artifact entry (one machine or user).
///
/// Represents an artifact that belongs to a specific target (NixOS machine
/// or home-manager user). Each target has its own independent copy.
#[derive(Debug, Clone)]
pub struct ArtifactEntry {
    /// Target name (e.g., "machine-one" or "alice@host")
    pub target: String,
    /// Type of target (NixOS machine or home-manager user)
    pub target_type: TargetType,
    /// The artifact definition (name, files, prompts, generator)
    pub artifact: ArtifactDef,
    /// Current generation status
    pub status: ArtifactStatus,
    /// Logs organized by generation step
    pub step_logs: StepLogs,
    /// Whether the artifact already exists in backend storage
    pub exists: bool,
}

/// Target type for artifact entries.
///
/// Determines the context (NixOS vs home-manager) and affects
/// how artifacts are serialized and which environment variables
/// are passed to scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
    /// NixOS machine configuration
    Nixos,
    /// Home-manager user configuration
    HomeManager,
}

impl TargetType {
    pub fn context_str(self) -> &'static str {
        match self {
            Self::Nixos => "nixos",
            Self::HomeManager => "homemanager",
        }
    }
}

impl std::fmt::Display for TargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.context_str())
    }
}

/// Current status of an artifact through its lifecycle.
///
/// The normal flow is `Pending` → (`NeedsGeneration` | `UpToDate`), where the
/// branch is decided by the backend's `check_serialization` step.  If the user
/// then triggers generation the status moves to `Generating` while the
/// generator and serialization steps run, and finally settles back to
/// `UpToDate`.  Any step can fail and move the status to `Failed`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ArtifactStatus {
    /// Initial state before the backend check has run.  Every entry starts
    /// here; the check step transitions it to either `NeedsGeneration` or
    /// `UpToDate`.
    #[default]
    Pending,
    /// The backend's `check_serialization` determined that the serialized
    /// artifact is stale or missing and needs to be regenerated.
    NeedsGeneration,
    /// The backend's `check_serialization` confirmed the serialized artifact
    /// is current, *or* generation and serialization have just completed
    /// successfully.
    UpToDate,
    /// The generator or serialization step is actively running for this
    /// artifact. The inner state tracks which step and any output.
    Generating(GeneratingSubstate),
    /// A step (check, generation, or serialization) failed.  The error
    /// message and output are preserved for display.
    Failed {
        error: String,
        output: String,
        retry_available: bool,
    },
}

/// Substate while an artifact is being generated.
/// Tracks the current step and accumulated output for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratingSubstate {
    /// Which step is currently running
    pub step: GenerationStep,
    /// Accumulated output from the generator (shown after completion)
    pub output: String,
}

impl Default for GeneratingSubstate {
    fn default() -> Self {
        Self {
            step: GenerationStep::CheckSerialization,
            output: String::new(),
        }
    }
}

/// The steps in the artifact generation process.
///
/// These steps are executed in order:
/// 1. CheckSerialization - Determine if regeneration is needed
/// 2. RunningGenerator - Execute the generator script
/// 3. Serializing - Store generated files in the backend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GenerationStep {
    /// Checking if generation is needed via check_serialization script
    #[default]
    CheckSerialization,
    /// Running the generator script to produce files
    RunningGenerator,
    /// Running the serialize script to store files in backend
    Serializing,
}

/// A single log entry for an artifact generation step.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Severity level of the log entry
    pub level: LogLevel,
    /// Log message content
    pub message: String,
}

/// Log severity/category for log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Summary messages
    Info,
    /// Stdout from generator/backend scripts
    Output,
    /// Stderr from generator/backend scripts
    Error,
    /// Completion/success messages
    Success,
}

/// Identifies which stream a line came from for streaming output.
#[derive(Debug, Clone, Copy)]
pub enum OutputStream {
    /// Standard output from a script
    Stdout,
    /// Standard error from a script
    Stderr,
}

impl From<crate::tui::channels::OutputStream> for OutputStream {
    fn from(stream: crate::tui::channels::OutputStream) -> Self {
        match stream {
            crate::tui::channels::OutputStream::Stdout => Self::Stdout,
            crate::tui::channels::OutputStream::Stderr => Self::Stderr,
        }
    }
}

/// A warning about backend capability issues (non-blocking)
#[derive(Debug, Clone)]
pub struct Warning {
    pub artifact_name: String,
    pub message: String,
}

/// The three generation steps that produce logs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum LogStep {
    #[default]
    Check,
    Generate,
    Serialize,
}

impl LogStep {
    pub fn next(self) -> Self {
        match self {
            Self::Check => Self::Generate,
            Self::Generate => Self::Serialize,
            Self::Serialize => Self::Check,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Check => "Check",
            Self::Generate => "Generate",
            Self::Serialize => "Serialize",
        }
    }
}

/// State for the chronological log view with expandable sections
#[derive(Debug, Clone)]
pub struct ChronologicalLogState {
    /// Index of the artifact being viewed
    pub artifact_index: usize,
    /// Artifact name for the header display
    pub artifact_name: String,
    /// Which sections are currently expanded (all steps by default)
    pub expanded_sections: HashSet<LogStep>,
    /// Vertical scroll offset in lines
    pub scroll_offset: usize,
    /// Currently focused section for keyboard navigation
    pub focused_section: Option<LogStep>,
}

impl ChronologicalLogState {
    /// Create a new chronological log state for viewing an artifact
    pub fn new(artifact_index: usize, artifact_name: String) -> Self {
        Self {
            artifact_index,
            artifact_name,
            expanded_sections: LogStep::all_steps().iter().cloned().collect(),
            scroll_offset: 0,
            focused_section: Some(LogStep::Check),
        }
    }

    /// Check if a specific section is expanded
    pub fn is_expanded(&self, step: LogStep) -> bool {
        self.expanded_sections.contains(&step)
    }

    /// Toggle a section's expanded state
    pub fn toggle_section(&mut self, step: LogStep) {
        if self.expanded_sections.contains(&step) {
            self.expanded_sections.remove(&step);
        } else {
            self.expanded_sections.insert(step);
        }
    }

    /// Expand all sections
    pub fn expand_all(&mut self) {
        self.expanded_sections = LogStep::all_steps().iter().cloned().collect();
    }

    /// Collapse all sections
    pub fn collapse_all(&mut self) {
        self.expanded_sections.clear();
    }

    /// Move focus to the next section
    pub fn focus_next(&mut self) {
        self.focused_section = self.focused_section.map(|s| s.next());
    }

    /// Move focus to the previous section
    pub fn focus_previous(&mut self) {
        self.focused_section = self.focused_section.map(|s| s.previous());
    }

    /// Scroll down by a number of lines
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset += lines;
    }

    /// Scroll up by a number of lines
    pub fn scroll_up(&mut self, lines: usize) {
        if lines > self.scroll_offset {
            self.scroll_offset = 0;
        } else {
            self.scroll_offset -= lines;
        }
    }

    /// Calculate the maximum scroll offset based on content and visible height
    pub fn max_scroll(&self, step_logs: &StepLogs) -> usize {
        let mut total_lines = 0usize;
        for step in LogStep::all_steps() {
            // One line for the section header
            total_lines += 1;
            // If expanded, add log lines
            if self.is_expanded(*step) {
                total_lines += step_logs.get(*step).len();
            }
        }
        total_lines
    }

    /// Clamp scroll offset to valid range
    pub fn clamp_scroll(&mut self, max_scroll: usize) {
        self.scroll_offset = self.scroll_offset.min(max_scroll);
    }
}

impl Default for ChronologicalLogState {
    fn default() -> Self {
        // All sections are expanded by default
        Self {
            artifact_index: 0,
            artifact_name: String::new(),
            expanded_sections: LogStep::all_steps().iter().cloned().collect(),
            scroll_offset: 0,
            focused_section: Some(LogStep::Check),
        }
    }
}

impl LogStep {
    /// Get all possible step variants
    pub fn all_steps() -> &'static [LogStep] {
        static STEPS: &[LogStep] = &[LogStep::Check, LogStep::Generate, LogStep::Serialize];
        STEPS
    }

    /// Get the previous step in the sequence, wrapping around
    pub fn previous(self) -> Self {
        match self {
            Self::Check => Self::Serialize,
            Self::Generate => Self::Check,
            Self::Serialize => Self::Generate,
        }
    }
}

/// Logs organized by step
#[derive(Debug, Clone, Default)]
pub struct StepLogs {
    pub check: Vec<LogEntry>,
    pub generate: Vec<LogEntry>,
    pub serialize: Vec<LogEntry>,
}

impl StepLogs {
    pub fn get(&self, step: LogStep) -> &Vec<LogEntry> {
        match step {
            LogStep::Check => &self.check,
            LogStep::Generate => &self.generate,
            LogStep::Serialize => &self.serialize,
        }
    }

    pub fn get_mut(&mut self, step: LogStep) -> &mut Vec<LogEntry> {
        match step {
            LogStep::Check => &mut self.check,
            LogStep::Generate => &mut self.generate,
            LogStep::Serialize => &mut self.serialize,
        }
    }

    /// Append stdout lines as Output-level entries
    pub fn append_stdout(&mut self, step: LogStep, lines: &[String]) {
        let entries = lines.iter().map(|line| LogEntry {
            level: LogLevel::Output,
            message: line.clone(),
        });
        self.get_mut(step).extend(entries);
    }

    /// Append stderr lines as Error-level entries
    pub fn append_stderr(&mut self, step: LogStep, lines: &[String]) {
        let entries = lines.iter().map(|line| LogEntry {
            level: LogLevel::Error,
            message: line.clone(),
        });
        self.get_mut(step).extend(entries);
    }
}

/// State for the prompt input screen.
///
/// Collects user input for artifact prompts before generation.
/// Supports three input modes: line, multiline, and hidden (for secrets).
#[derive(Debug, Clone)]
pub struct PromptState {
    /// Index of the artifact being prompted for
    pub artifact_index: usize,
    /// Artifact name for display
    pub artifact_name: String,
    /// Artifact description (optional, for display)
    pub description: Option<String>,
    /// List of prompts to collect
    pub prompts: Vec<PromptEntry>,
    /// Index of the current prompt being collected
    pub current_prompt_index: usize,
    /// Current input mode (line/multiline/hidden)
    pub input_mode: InputMode,
    /// Current input buffer
    pub buffer: String,
    /// Already collected prompt values (name -> value)
    pub collected: HashMap<String, String>,
}

impl PromptState {
    pub fn current_prompt(&self) -> Option<&PromptEntry> {
        self.prompts.get(self.current_prompt_index)
    }

    pub fn is_complete(&self) -> bool {
        self.current_prompt_index >= self.prompts.len()
    }

    pub fn progress(&self) -> (usize, usize) {
        (self.current_prompt_index + 1, self.prompts.len())
    }
}

/// A single prompt definition
#[derive(Debug, Clone)]
pub struct PromptEntry {
    pub name: String,
    pub description: Option<String>,
}

/// Input mode for collecting prompt values.
///
/// - `Line`: Single line input (Enter submits)
/// - `Multiline`: Multi-line input (Ctrl+D submits, Enter adds newline)
/// - `Hidden`: Password-style input (characters hidden)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Single line input (Enter submits)
    #[default]
    Line,
    /// Multi-line input (Ctrl+D submits)
    Multiline,
    /// Password-style hidden input
    Hidden,
}

impl InputMode {
    pub fn next(self) -> Self {
        match self {
            Self::Line => Self::Multiline,
            Self::Multiline => Self::Hidden,
            Self::Hidden => Self::Line,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::Multiline => "multiline",
            Self::Hidden => "hidden",
        }
    }
}

/// State while generating an artifact (screen state)
#[derive(Debug, Clone)]
pub struct GeneratingState {
    pub artifact_index: usize,
    pub artifact_name: String,
    pub step: GenerationStep,
    pub log_lines: Vec<String>,
    /// true if regenerating existing artifact, false if creating new
    pub exists: bool,
}

/// State when generation is complete
#[derive(Debug, Clone, Default)]
pub struct DoneState {
    pub generated_count: usize,
    pub skipped_count: usize,
    pub failed: Vec<String>,
}

/// State for the regeneration confirmation dialog
#[derive(Debug, Clone)]
pub struct ConfirmRegenerateState {
    pub artifact_index: usize,
    pub artifact_name: String,
    /// Description of affected targets (for shared artifacts)
    pub affected_targets: Vec<String>,
    /// true = Leave selected (safe), false = Regenerate selected
    pub leave_selected: bool,
}

/// State for the generator selection screen (for shared artifacts with multiple generators)
#[derive(Debug, Clone)]
pub struct SelectGeneratorState {
    pub artifact_index: usize,
    pub artifact_name: String,
    /// artifact description (optional, for display in dialog)
    pub description: Option<String>,
    pub generators: Vec<GeneratorInfo>,
    pub selected_index: usize,
    /// Prompts required for this artifact (from config::make::PromptDef)
    pub prompts: Vec<crate::config::make::PromptDef>,
    /// NixOS machine names that use this artifact
    pub nixos_targets: Vec<String>,
    /// Home-manager user identifiers that use this artifact
    pub home_targets: Vec<String>,
}

impl SelectGeneratorState {
    pub fn selected_generator(&self) -> Option<&GeneratorInfo> {
        self.generators.get(self.selected_index)
    }
}

/// An entry in the artifact list that can be either per-target or shared.
///
/// This enum unifies the artifact list display, handling both:
/// - Single artifacts (one per target)
/// - Shared artifacts (one artifact shared across multiple targets)
#[derive(Debug, Clone)]
pub enum ListEntry {
    /// Per-target artifact (one machine or user)
    Single(ArtifactEntry),
    /// Shared artifact across multiple targets
    Shared(SharedEntry),
}

impl ListEntry {
    pub fn artifact_name(&self) -> &str {
        match self {
            ListEntry::Single(entry) => &entry.artifact.name,
            ListEntry::Shared(entry) => &entry.info.artifact_name,
        }
    }

    pub fn status(&self) -> &ArtifactStatus {
        match self {
            ListEntry::Single(entry) => &entry.status,
            ListEntry::Shared(entry) => &entry.status,
        }
    }

    pub fn status_mut(&mut self) -> &mut ArtifactStatus {
        match self {
            ListEntry::Single(entry) => &mut entry.status,
            ListEntry::Shared(entry) => &mut entry.status,
        }
    }

    pub fn step_logs(&self) -> &StepLogs {
        match self {
            ListEntry::Single(entry) => &entry.step_logs,
            ListEntry::Shared(entry) => &entry.step_logs,
        }
    }

    pub fn step_logs_mut(&mut self) -> &mut StepLogs {
        match self {
            ListEntry::Single(entry) => &mut entry.step_logs,
            ListEntry::Shared(entry) => &mut entry.step_logs,
        }
    }

    pub fn is_shared(&self) -> bool {
        matches!(self, ListEntry::Shared(_))
    }
}

/// A shared artifact entry in the list
#[derive(Debug, Clone)]
pub struct SharedEntry {
    pub info: SharedArtifactInfo,
    pub status: ArtifactStatus,
    pub step_logs: StepLogs,
    /// The selected generator path (set after user selection)
    pub selected_generator: Option<String>,
    /// Whether the artifact already exists in backend storage
    pub exists: bool,
}

impl ArtifactStatus {
    /// Get the display symbol for this status
    pub fn symbol(&self) -> &'static str {
        match self {
            ArtifactStatus::Pending => "○",
            ArtifactStatus::NeedsGeneration => "!",
            ArtifactStatus::UpToDate => "✓",
            ArtifactStatus::Generating(_) => "⟳",
            ArtifactStatus::Failed { .. } => "✗",
        }
    }

    /// Get the display style (color) for this status
    pub fn style(&self) -> Style {
        match self {
            ArtifactStatus::Pending => Style::default().fg(Color::Gray),
            ArtifactStatus::NeedsGeneration => Style::default().fg(Color::Yellow),
            ArtifactStatus::UpToDate => Style::default().fg(Color::Green),
            ArtifactStatus::Generating(_) => Style::default().fg(Color::Cyan),
            ArtifactStatus::Failed { .. } => Style::default().fg(Color::Red),
        }
    }

    /// Check if this status is currently generating
    pub fn is_generating(&self) -> bool {
        matches!(self, ArtifactStatus::Generating(_))
    }

    /// Check if generation can be started from this status
    pub fn can_generate(&self) -> bool {
        matches!(
            self,
            ArtifactStatus::Pending
                | ArtifactStatus::NeedsGeneration
                | ArtifactStatus::Failed { .. }
        )
    }
}

impl GenerationStep {
    /// Get a human-readable description of this step
    pub fn description(&self) -> &'static str {
        match self {
            GenerationStep::CheckSerialization => "CheckSerialization...",
            GenerationStep::RunningGenerator => "Running generator...",
            GenerationStep::Serializing => "Serializing...",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_mode_cycles() {
        let mode = InputMode::Line;
        assert_eq!(mode.next(), InputMode::Multiline);
        assert_eq!(mode.next().next(), InputMode::Hidden);
        assert_eq!(mode.next().next().next(), InputMode::Line);
    }

    #[test]
    fn test_input_mode_labels() {
        assert_eq!(InputMode::Line.label(), "line");
        assert_eq!(InputMode::Multiline.label(), "multiline");
        assert_eq!(InputMode::Hidden.label(), "hidden");
    }

    #[test]
    fn test_target_type_context_str() {
        assert_eq!(TargetType::Nixos.context_str(), "nixos");
        assert_eq!(TargetType::HomeManager.context_str(), "homemanager");
    }

    #[test]
    fn test_log_step_cycles() {
        let step = LogStep::Check;
        assert_eq!(step.next(), LogStep::Generate);
        assert_eq!(step.next().next(), LogStep::Serialize);
        assert_eq!(step.next().next().next(), LogStep::Check);
    }

    #[test]
    fn test_log_step_labels() {
        assert_eq!(LogStep::Check.label(), "Check");
        assert_eq!(LogStep::Generate.label(), "Generate");
        assert_eq!(LogStep::Serialize.label(), "Serialize");
    }
}
