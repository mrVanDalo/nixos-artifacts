use crate::config::make::{ArtifactDef, GeneratorInfo, SharedArtifactInfo};
use ratatui::style::{Color, Style};
use std::collections::HashMap;

/// Root application state
#[derive(Debug, Clone, Default)]
pub struct Model {
    pub screen: Screen,
    /// Legacy field - list of per-target artifacts
    pub artifacts: Vec<ArtifactEntry>,
    /// New field - unified list of entries (single and shared)
    pub entries: Vec<ListEntry>,
    pub selected_index: usize,
    pub selected_log_step: LogStep,
    pub error: Option<String>,
    /// Non-blocking warnings about backend capability issues
    pub warnings: Vec<Warning>,
    /// Animation frame counter for spinner animation
    pub tick_count: usize,
}

/// Current screen/view being displayed
#[derive(Debug, Clone, Default)]
pub enum Screen {
    #[default]
    ArtifactList,
    SelectGenerator(SelectGeneratorState),
    ConfirmRegenerate(ConfirmRegenerateState),
    Prompt(PromptState),
    Generating(GeneratingState),
    Done(DoneState),
}

/// An artifact with its associated machine/user and status
#[derive(Debug, Clone)]
pub struct ArtifactEntry {
    pub target: String,
    pub target_type: TargetType,
    pub artifact: ArtifactDef,
    pub status: ArtifactStatus,
    pub step_logs: StepLogs,
    /// Whether the artifact already exists in backend storage
    pub exists: bool,
}

/// Whether this is a NixOS machine or home-manager user
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
    Nixos,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationStep {
    /// Checking if generation is needed via check_serialization script
    CheckSerialization,
    /// Running the generator script to produce files
    RunningGenerator,
    /// Running the serialize script to store files in backend
    Serializing,
}

impl Default for GenerationStep {
    fn default() -> Self {
        GenerationStep::CheckSerialization
    }
}

/// A single log entry for an artifact
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
}

/// Log severity/category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,    // Summary messages
    Output,  // Stdout from scripts
    Error,   // Stderr from scripts
    Success, // Completion messages
}

/// Identifies which stream a line came from for streaming output
#[derive(Debug, Clone, Copy)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

impl From<crate::tui::channels::OutputStream> for OutputStream {
    fn from(stream: crate::tui::channels::OutputStream) -> Self {
        match stream {
            crate::tui::channels::OutputStream::Stdout => OutputStream::Stdout,
            crate::tui::channels::OutputStream::Stderr => OutputStream::Stderr,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

/// State for the prompt input screen
#[derive(Debug, Clone)]
pub struct PromptState {
    pub artifact_index: usize,
    pub artifact_name: String,
    /// artifact description (optional, for display in prompt)
    pub description: Option<String>,
    pub prompts: Vec<PromptEntry>,
    pub current_prompt_index: usize,
    pub input_mode: InputMode,
    pub buffer: String,
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

/// Input mode for prompts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Line,
    Multiline,
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

/// An entry in the artifact list that can be either per-target or shared
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
