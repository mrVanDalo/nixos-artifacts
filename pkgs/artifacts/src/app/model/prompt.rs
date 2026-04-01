//! Prompt input types for collecting user input during artifact generation.

use std::collections::HashMap;

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
}
