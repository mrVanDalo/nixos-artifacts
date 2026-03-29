//! Message types for the Elm Architecture update loop.
//!
//! Messages are the only way to trigger state changes in the application.
//! They flow from the event source (terminal input, background tasks, timers)
//! through the update function to produce new state.
//!
//! # Message Sources
//!
//! - **User input**: `KeyEvent` messages from terminal
//! - **Background tasks**: Results from generator/serialization scripts
//! - **Timer**: `Tick` messages for animations
//! - **Navigation**: Log view messages for scrolling and expansion

use crossterm::event::{KeyCode, KeyModifiers};

/// All possible events/messages in the application.
///
/// This enum represents every possible event that can cause a state change.
/// The update function in [`crate::app::update()`] matches on these messages to compute new state.
#[derive(Debug, Clone)]
pub enum Message {
    /// Keyboard input
    Key(KeyEvent),

    /// Timer tick (for animations, polling)
    Tick,

    /// Check serialization completed for an artifact
    CheckSerializationResult {
        artifact_index: usize,
        status: crate::app::model::ArtifactStatus,
        result: Result<ScriptOutput, String>,
    },

    /// Generator script finished (per-target artifact).
    GeneratorFinished {
        artifact_index: usize,
        result: Result<ScriptOutput, String>,
    },

    /// Serialize script finished
    SerializeFinished {
        artifact_index: usize,
        result: Result<ScriptOutput, String>,
    },

    /// Generator selected for a shared artifact
    GeneratorSelected {
        artifact_index: usize,
        generator_path: String,
    },

    /// Shared check serialization completed for an artifact
    SharedCheckSerializationResult {
        artifact_index: usize,
        statuses: Vec<crate::app::model::ArtifactStatus>,
        outputs: Vec<ScriptOutput>,
    },

    /// Shared generator script finished
    SharedGeneratorFinished {
        artifact_index: usize,
        result: Result<ScriptOutput, String>,
    },

    /// Shared serialize script finished
    SharedSerializeFinished {
        artifact_index: usize,
        results: Vec<(String, bool, ScriptOutput)>,
    },

    /// Streaming output line received during script execution
    OutputLine {
        artifact_index: usize,
        stream: crate::app::model::OutputStream,
        content: String,
    },

    /// Toggle expansion state of a log section (chronological log view)
    ToggleSection { step: crate::app::model::LogStep },

    /// Scroll log content (chronological log view)
    ScrollLogs { delta: i32 },

    /// Expand all log sections (chronological log view)
    ExpandAllSections,

    /// Collapse all log sections (chronological log view)
    CollapseAllSections,

    /// Navigate to next section in chronological log view
    FocusNextSection,

    /// Navigate to previous section in chronological log view
    FocusPreviousSection,

    /// Request to quit the application
    Quit,
}

/// Output captured from script execution (stdout/stderr)
#[derive(Debug, Clone, Default)]
pub struct ScriptOutput {
    pub stdout_lines: Vec<String>,
    pub stderr_lines: Vec<String>,
}

impl ScriptOutput {
    /// Convert from CapturedOutput to ScriptOutput, copying stdout and stderr
    pub fn from_captured(captured: &crate::backend::output_capture::CapturedOutput) -> Self {
        Self {
            stdout_lines: captured.stdout.clone(),
            stderr_lines: captured.stderr.clone(),
        }
    }

    /// Create a ScriptOutput from a single message (for errors/warnings)
    pub fn from_message(message: &str) -> Self {
        Self {
            stdout_lines: vec![message.to_string()],
            stderr_lines: Vec::new(),
        }
    }
}

/// Wrapper around crossterm key events for easier construction in tests.
///
/// Provides convenient constructors for common key events, making tests
/// more readable and easier to write.
///
/// # Examples
///
/// ```rust,ignore
/// let enter = KeyEvent::enter();
/// let ctrl_c = KeyEvent::ctrl('c');
/// let letter = KeyEvent::char('a');
/// ```
#[derive(Debug, Clone)]
pub struct KeyEvent {
    /// The key code (character, special key, etc.)
    pub code: KeyCode,
    /// Modifier keys held during the event
    pub modifiers: KeyModifiers,
}

impl KeyEvent {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub fn from_code(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::NONE)
    }

    pub fn char(c: char) -> Self {
        Self::from_code(KeyCode::Char(c))
    }

    pub fn ctrl(c: char) -> Self {
        Self::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    pub fn enter() -> Self {
        Self::from_code(KeyCode::Enter)
    }

    pub fn tab() -> Self {
        Self::from_code(KeyCode::Tab)
    }

    pub fn esc() -> Self {
        Self::from_code(KeyCode::Esc)
    }

    pub fn backspace() -> Self {
        Self::from_code(KeyCode::Backspace)
    }

    pub fn up() -> Self {
        Self::from_code(KeyCode::Up)
    }

    pub fn down() -> Self {
        Self::from_code(KeyCode::Down)
    }
}

impl From<crossterm::event::KeyEvent> for KeyEvent {
    fn from(event: crossterm::event::KeyEvent) -> Self {
        Self {
            code: event.code,
            modifiers: event.modifiers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_event_constructors() {
        let key = KeyEvent::char('a');
        assert_eq!(key.code, KeyCode::Char('a'));
        assert_eq!(key.modifiers, KeyModifiers::NONE);

        let ctrl_c = KeyEvent::ctrl('c');
        assert_eq!(ctrl_c.code, KeyCode::Char('c'));
        assert_eq!(ctrl_c.modifiers, KeyModifiers::CONTROL);

        let enter = KeyEvent::enter();
        assert_eq!(enter.code, KeyCode::Enter);
    }
}
