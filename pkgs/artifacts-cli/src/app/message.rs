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
        needs_generation: bool,
    },

    /// Generator script finished
    GeneratorFinished {
        artifact_index: usize,
        result: Result<(), String>,
    },

    /// Serialize script finished
    SerializeFinished {
        artifact_index: usize,
        result: Result<(), String>,
    },

    /// Request to quit the application
    Quit,
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
