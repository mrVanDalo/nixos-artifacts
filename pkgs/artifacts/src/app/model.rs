use crate::config::make::ArtifactDef;
use std::collections::HashMap;

/// Root application state
#[derive(Debug, Clone, Default)]
pub struct Model {
    pub screen: Screen,
    pub artifacts: Vec<ArtifactEntry>,
    pub selected_index: usize,
    pub selected_log_step: LogStep,
    pub error: Option<String>,
}

/// Current screen/view being displayed
#[derive(Debug, Clone, Default)]
pub enum Screen {
    #[default]
    ArtifactList,
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

/// Current status of an artifact
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ArtifactStatus {
    #[default]
    Pending,
    NeedsGeneration,
    UpToDate,
    Generating,
    Failed(String),
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
}

/// State for the prompt input screen
#[derive(Debug, Clone)]
pub struct PromptState {
    pub artifact_index: usize,
    pub artifact_name: String,
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

/// State while generating an artifact
#[derive(Debug, Clone)]
pub struct GeneratingState {
    pub artifact_index: usize,
    pub artifact_name: String,
    pub step: GenerationStep,
    pub log_lines: Vec<String>,
}

/// Current step in the generation process
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationStep {
    RunningGenerator,
    Serializing,
}

/// State when generation is complete
#[derive(Debug, Clone, Default)]
pub struct DoneState {
    pub generated_count: usize,
    pub skipped_count: usize,
    pub failed: Vec<String>,
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
