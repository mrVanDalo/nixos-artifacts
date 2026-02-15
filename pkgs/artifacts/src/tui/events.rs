use crate::app::message::{KeyEvent, Msg};
use crossterm::event::{self, Event, KeyEventKind};
use std::collections::VecDeque;
use std::time::Duration;

/// Trait for sources of application events.
/// This abstraction allows injecting test events in tests.
pub trait EventSource {
    /// Get the next event, if available.
    /// Returns None when the event source is exhausted.
    fn next_event(&mut self) -> Option<Msg>;

    /// Check if an event is available without consuming it.
    /// Returns true if next_event() would return immediately.
    fn has_event(&mut self) -> bool;
}

/// Production event source that reads from the terminal via crossterm.
pub struct TerminalEventSource {
    tick_rate: Duration,
}

impl TerminalEventSource {
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    pub fn default_tick_rate() -> Duration {
        Duration::from_millis(50)
    }
}

impl Default for TerminalEventSource {
    fn default() -> Self {
        Self::new(Self::default_tick_rate())
    }
}

impl EventSource for TerminalEventSource {
    fn next_event(&mut self) -> Option<Msg> {
        if event::poll(self.tick_rate).ok()? {
            match event::read().ok()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    Some(Msg::Key(KeyEvent::from(key)))
                }
                _ => Some(Msg::Tick),
            }
        } else {
            Some(Msg::Tick)
        }
    }

    fn has_event(&mut self) -> bool {
        // Check if an event is available without consuming it
        event::poll(std::time::Duration::from_secs(0)).unwrap_or(false)
    }
}

/// Test event source that replays a scripted sequence of events.
/// Used for deterministic testing without terminal interaction.
#[derive(Debug, Default, Clone)]
pub struct ScriptedEventSource {
    events: VecDeque<Msg>,
}

impl ScriptedEventSource {
    pub fn new(events: impl IntoIterator<Item = Msg>) -> Self {
        Self {
            events: events.into_iter().collect(),
        }
    }

    /// Create from a sequence of key events
    pub fn from_keys(keys: impl IntoIterator<Item = KeyEvent>) -> Self {
        Self::new(keys.into_iter().map(Msg::Key))
    }

    /// Check if there are remaining events
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get the number of remaining events
    pub fn len(&self) -> usize {
        self.events.len()
    }
}

impl EventSource for ScriptedEventSource {
    fn next_event(&mut self) -> Option<Msg> {
        self.events.pop_front()
    }

    fn has_event(&mut self) -> bool {
        !self.events.is_empty()
    }
}

// ============================================================================
// Test Helpers
// ============================================================================

/// Helper functions for creating test events
pub mod test_helpers {
    use super::*;
    use crossterm::event::KeyCode;

    /// Create a key message from a KeyCode
    pub fn key(code: KeyCode) -> Msg {
        Msg::Key(KeyEvent::from_code(code))
    }

    /// Create a character key message
    pub fn char(c: char) -> Msg {
        Msg::Key(KeyEvent::char(c))
    }

    /// Create a Ctrl+key message
    pub fn ctrl(c: char) -> Msg {
        Msg::Key(KeyEvent::ctrl(c))
    }

    /// Create messages for typing a string
    pub fn type_string(s: &str) -> Vec<Msg> {
        s.chars().map(char).collect()
    }

    /// Create an Enter key message
    pub fn enter() -> Msg {
        Msg::Key(KeyEvent::enter())
    }

    /// Create an Escape key message
    pub fn esc() -> Msg {
        Msg::Key(KeyEvent::esc())
    }

    /// Create a Tab key message
    pub fn tab() -> Msg {
        Msg::Key(KeyEvent::tab())
    }

    /// Create a Backspace key message
    pub fn backspace() -> Msg {
        Msg::Key(KeyEvent::backspace())
    }

    /// Create an Up arrow key message
    pub fn up() -> Msg {
        Msg::Key(KeyEvent::up())
    }

    /// Create a Down arrow key message
    pub fn down() -> Msg {
        Msg::Key(KeyEvent::down())
    }

    /// Build a sequence of events for a complete prompt submission
    pub fn submit_prompt(value: &str) -> Vec<Msg> {
        let mut events = type_string(value);
        events.push(enter());
        events
    }

    /// Build a sequence for a hidden mode prompt
    pub fn submit_hidden_prompt(value: &str) -> Vec<Msg> {
        let mut events = vec![tab(), tab()]; // Line -> Multiline -> Hidden
        events.extend(type_string(value));
        events.push(enter());
        events
    }
}

#[cfg(test)]
mod tests {
    use super::test_helpers::*;
    use super::*;

    #[test]
    fn test_scripted_event_source() {
        let mut source = ScriptedEventSource::new(vec![char('a'), char('b'), enter()]);

        assert_eq!(source.len(), 3);
        assert!(!source.is_empty());

        assert!(matches!(source.next_event(), Some(Msg::Key(_))));
        assert!(matches!(source.next_event(), Some(Msg::Key(_))));
        assert!(matches!(source.next_event(), Some(Msg::Key(_))));
        assert!(source.next_event().is_none());
        assert!(source.is_empty());
    }

    #[test]
    fn test_type_string_helper() {
        let events = type_string("hi");
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_submit_prompt_helper() {
        let events = submit_prompt("secret");
        assert_eq!(events.len(), 7); // 6 chars + enter
    }
}
