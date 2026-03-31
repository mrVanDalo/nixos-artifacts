//! Test helper functions for creating test events.
//!
//! This module provides convenient functions for creating key events
//! in tests without requiring actual terminal interaction.

#![allow(dead_code)]

use artifacts::app::KeyEvent;
use artifacts::app::message::Message;
use crossterm::event::KeyCode;

pub fn key(code: KeyCode) -> Message {
    Message::Key(KeyEvent::from_code(code))
}

pub fn char(c: char) -> Message {
    Message::Key(KeyEvent::char(c))
}

pub fn ctrl(c: char) -> Message {
    Message::Key(KeyEvent::ctrl(c))
}

pub fn type_string(s: &str) -> Vec<Message> {
    s.chars().map(char).collect()
}

pub fn enter() -> Message {
    Message::Key(KeyEvent::enter())
}

pub fn esc() -> Message {
    Message::Key(KeyEvent::esc())
}

pub fn tab() -> Message {
    Message::Key(KeyEvent::tab())
}

pub fn backspace() -> Message {
    Message::Key(KeyEvent::backspace())
}

pub fn up() -> Message {
    Message::Key(KeyEvent::up())
}

pub fn down() -> Message {
    Message::Key(KeyEvent::down())
}

pub fn submit_prompt(value: &str) -> Vec<Message> {
    let mut events = type_string(value);
    events.push(enter());
    events
}

pub fn submit_hidden_prompt(value: &str) -> Vec<Message> {
    let mut events = vec![tab(), tab()];
    events.extend(type_string(value));
    events.push(enter());
    events
}

#[test]
fn test_type_string_helper() {
    let events = type_string("hi");
    assert_eq!(events.len(), 2);
}

#[test]
fn test_submit_prompt_helper() {
    let events = submit_prompt("secret");
    assert_eq!(events.len(), 7);
}

#[test]
fn test_scripted_event_source() {
    use artifacts::tui::EventSource;
    use artifacts::tui::events::ScriptedEventSource;

    let mut source = ScriptedEventSource::new(vec![char('a'), char('b'), enter()]);

    assert_eq!(source.len(), 3);
    assert!(!source.is_empty());

    assert!(matches!(source.next_event(), Some(Message::Key(_))));
    assert!(matches!(source.next_event(), Some(Message::Key(_))));
    assert!(matches!(source.next_event(), Some(Message::Key(_))));
    assert!(source.next_event().is_none());
    assert!(source.is_empty());
}
