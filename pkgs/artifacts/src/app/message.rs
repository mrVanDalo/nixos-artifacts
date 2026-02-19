use crossterm::event::{KeyCode, KeyModifiers};

/// All possible events/messages in the application
#[derive(Debug, Clone)]
pub enum Msg {
    /// Keyboard input
    Key(KeyEvent),

    /// Timer tick (for animations, polling)
    Tick,

    /// Check serialization completed for an artifact
    CheckSerializationResult {
        artifact_index: usize,
        needs_generation: bool,      // true = artifact needs to be regenerated
        exists: bool,                // true = artifact already exists in backend
        result: Result<(), String>,  // Err = check failed, Ok = check succeeded
        output: Option<CheckOutput>, // Captured stdout/stderr from the check script
    },

    /// Generator script finished
    GeneratorFinished {
        artifact_index: usize,
        result: Result<GeneratorOutput, String>,
    },

    /// Serialize script finished
    SerializeFinished {
        artifact_index: usize,
        result: Result<SerializeOutput, String>,
    },

    /// Generator selected for a shared artifact
    GeneratorSelected {
        artifact_index: usize,
        generator_path: String,
    },

    /// Shared check serialization completed for an artifact
    SharedCheckSerializationResult {
        artifact_index: usize,
        needs_generation: bool,      // true = artifact needs to be regenerated
        exists: bool,                // true = artifact already exists in backend
        result: Result<(), String>,  // Err = check failed, Ok = check succeeded
        output: Option<CheckOutput>, // Captured stdout/stderr from the check script
    },

    /// Shared generator script finished
    SharedGeneratorFinished {
        artifact_index: usize,
        result: Result<GeneratorOutput, String>,
    },

    /// Shared serialize script finished
    SharedSerializeFinished {
        artifact_index: usize,
        result: Result<SerializeOutput, String>,
    },

    /// Streaming output line received during script execution
    OutputLine {
        artifact_index: usize,
        stream: crate::app::model::OutputStream,
        content: String,
    },

    /// Request to quit the application
    Quit,

    /// Effect result from background task (contains EffectResult from channels)
    /// Note: This wraps the EffectResult from the channels module
    ChannelResult(crate::tui::channels::EffectResult),
}

/// Output captured from generator script execution
#[derive(Debug, Clone)]
pub struct GeneratorOutput {
    pub stdout_lines: Vec<String>,
    pub stderr_lines: Vec<String>,
    pub files_generated: usize,
}

/// Output captured from serialization script execution
#[derive(Debug, Clone)]
pub struct SerializeOutput {
    pub stdout_lines: Vec<String>,
    pub stderr_lines: Vec<String>,
}

/// Output captured from check_serialization script execution
#[derive(Debug, Clone)]
pub struct CheckOutput {
    pub stdout_lines: Vec<String>,
    pub stderr_lines: Vec<String>,
}

/// Wrapper around crossterm key events for easier construction in tests
#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub code: KeyCode,
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
