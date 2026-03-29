//! Event sources for the TUI application.
//!
//! This module provides abstractions for event input, allowing both
//! production terminal events and scripted test events.
//!
//! # Event Source Abstraction
//!
//! The [`EventSource`] trait abstracts over different event sources:
//! - [`TerminalEventSource`]: Reads real keyboard input from the terminal
//! - [`ScriptedEventSource`]: Provides scripted events for testing
//!
//! This abstraction enables testing the update logic without requiring
//! actual terminal interaction.

use crate::app::message::{KeyEvent, Message};
use crossterm::event::{self, Event, KeyEventKind};
use std::collections::VecDeque;
use std::time::Duration;

/// Trait for sources of application events.
///
/// This abstraction allows injecting test events in tests, making
/// the TUI logic testable without actual terminal interaction.
///
/// # Implementations
///
/// - [`TerminalEventSource`]: Production implementation reading from terminal
/// - [`ScriptedEventSource`]: Test implementation with predefined events
pub trait EventSource {
    /// Get the next event, if available.
    /// Returns None when the event source is exhausted.
    fn next_event(&mut self) -> Option<Message>;

    /// Check if an event is available without consuming it.
    /// Returns true if next_event() would return immediately.
    fn has_event(&mut self) -> bool;

    /// Check if the event source is permanently exhausted.
    /// Returns true when the source will never produce more events.
    /// For terminal sources, this always returns false.
    /// For scripted/test sources, this returns true when all events are consumed.
    fn is_exhausted(&mut self) -> bool;
}

/// Production event source that reads from the terminal via crossterm.
///
/// Polls the terminal for keyboard events at a configured tick rate.
/// Returns `Message::Tick` when no key events are available.
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
    fn next_event(&mut self) -> Option<Message> {
        if event::poll(self.tick_rate).ok()? {
            match event::read().ok()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    Some(Message::Key(KeyEvent::from(key)))
                }
                _ => Some(Message::Tick),
            }
        } else {
            Some(Message::Tick)
        }
    }

    fn has_event(&mut self) -> bool {
        // Check if an event is available without consuming it
        event::poll(std::time::Duration::from_secs(0)).unwrap_or(false)
    }

    fn is_exhausted(&mut self) -> bool {
        // Terminal source never exhausts - it always produces Tick events
        false
    }
}

/// Test event source that replays a scripted sequence of events.
/// Used for deterministic testing without terminal interaction.
#[derive(Debug, Default, Clone)]
pub struct ScriptedEventSource {
    events: VecDeque<Message>,
}

impl ScriptedEventSource {
    pub fn new(events: impl IntoIterator<Item = Message>) -> Self {
        Self {
            events: events.into_iter().collect(),
        }
    }

    /// Create from a sequence of key events
    pub fn from_keys(keys: impl IntoIterator<Item = KeyEvent>) -> Self {
        Self::new(keys.into_iter().map(Message::Key))
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
    fn next_event(&mut self) -> Option<Message> {
        self.events.pop_front()
    }

    fn has_event(&mut self) -> bool {
        !self.events.is_empty()
    }

    fn is_exhausted(&mut self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scripted_event_source() {
        let mut source = ScriptedEventSource::new(vec![
            Message::Key(KeyEvent::char('a')),
            Message::Key(KeyEvent::char('b')),
            Message::Key(KeyEvent::enter()),
        ]);

        assert_eq!(source.len(), 3);
        assert!(!source.is_empty());

        assert!(matches!(source.next_event(), Some(Message::Key(_))));
        assert!(matches!(source.next_event(), Some(Message::Key(_))));
        assert!(matches!(source.next_event(), Some(Message::Key(_))));
        assert!(source.next_event().is_none());
        assert!(source.is_empty());
    }
}
