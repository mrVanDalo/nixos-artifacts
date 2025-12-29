use crate::config::make::ArtifactDef;
use std::collections::HashMap;

/// Root application state
#[derive(Debug, Clone, Default)]
pub struct Model {
    pub screen: Screen,
    pub artifacts: Vec<ArtifactEntry>,
    pub selected_index: usize,
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
}
