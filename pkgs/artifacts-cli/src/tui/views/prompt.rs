use crate::app::model::{InputMode, PromptState};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the prompt input view
pub fn render_prompt(frame: &mut Frame, state: &PromptState, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Header
        Constraint::Length(4), // Description
        Constraint::Length(3), // Input
        Constraint::Min(1),    // Help text
    ])
    .split(area);

    // Header with artifact name and progress
    render_header(frame, state, chunks[0]);

    // Current prompt description
    render_description(frame, state, chunks[1]);

    // Input line
    render_input(frame, state, chunks[2]);

    // Help text
    render_help(frame, state, chunks[3]);
}

fn render_header(frame: &mut Frame, state: &PromptState, area: Rect) {
    let (current, total) = state.progress();
    let header_text = format!("Prompt: {} [{}/{}]", state.artifact_name, current, total);

    let header = Paragraph::new(header_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(header, area);
}

fn render_description(frame: &mut Frame, state: &PromptState, area: Rect) {
    let Some(prompt) = state.current_prompt() else {
        return;
    };

    let desc = prompt.description.as_deref().unwrap_or("No description");

    let description = Paragraph::new(vec![
        Line::styled(
            format!("Field: {}", prompt.name),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::from(desc),
    ])
    .block(Block::default().borders(Borders::NONE));

    frame.render_widget(description, area);
}

fn render_input(frame: &mut Frame, state: &PromptState, area: Rect) {
    let mode_style = match state.input_mode {
        InputMode::Line => Style::default().fg(Color::Green),
        InputMode::Multiline => Style::default().fg(Color::Yellow),
        InputMode::Hidden => Style::default().fg(Color::Red),
    };

    let display_buffer = if state.input_mode == InputMode::Hidden {
        "*".repeat(state.buffer.len())
    } else {
        state.buffer.clone()
    };

    let input_line = Line::from(vec![
        Span::styled(format!("[{}]", state.input_mode.label()), mode_style),
        Span::raw(" > "),
        Span::raw(&display_buffer),
        Span::styled("█", Style::default().fg(Color::Gray)), // Cursor
    ]);

    let input = Paragraph::new(vec![input_line])
        .block(Block::default().borders(Borders::ALL).title("Input"));

    frame.render_widget(input, area);
}

fn render_help(frame: &mut Frame, state: &PromptState, area: Rect) {
    let help_text = if state.buffer.is_empty() {
        match state.input_mode {
            InputMode::Line => "Tab: change mode | Enter: submit | Esc: cancel",
            InputMode::Multiline => "Tab: change mode | Ctrl+D: submit | Enter: newline | Esc: cancel",
            InputMode::Hidden => "Tab: change mode | Enter: submit | Esc: cancel",
        }
    } else {
        match state.input_mode {
            InputMode::Line => "Enter: submit | Esc: cancel",
            InputMode::Multiline => "Ctrl+D: submit | Enter: newline | Esc: cancel",
            InputMode::Hidden => "Enter: submit | Esc: cancel",
        }
    };

    let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));

    frame.render_widget(help, area);
}
