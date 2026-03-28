use crate::app::model::{InputMode, PromptState};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn render_prompt(frame: &mut Frame, state: &PromptState, area: Rect) {
    let input_height = if state.input_mode == InputMode::Multiline {
        Constraint::Min(5)
    } else {
        Constraint::Length(3)
    };

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(4),
        input_height,
        Constraint::Length(2),
    ])
    .split(area);

    render_header(frame, state, chunks[0]);
    render_description(frame, state, chunks[1]);
    render_input(frame, state, chunks[2]);
    render_help(frame, state, chunks[3]);
}

fn render_header(frame: &mut Frame, state: &PromptState, area: Rect) {
    let (current, total) = state.progress();
    let header_text = format!("Prompt: {} [{}/{}]", state.artifact_name, current, total);

    let header = Paragraph::new(header_text)
        .style(Style::default().add_modifier(Modifier::BOLD))
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
    let display_buffer = if state.input_mode == InputMode::Hidden {
        "*".repeat(state.buffer.len())
    } else {
        state.buffer.clone()
    };

    let title = format!("Input [{}]", state.input_mode.label());

    if state.input_mode == InputMode::Multiline {
        let lines: Vec<Line> = display_buffer
            .lines()
            .chain(std::iter::once(""))
            .map(|line| Line::from(line.to_string()))
            .collect();

        let input =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));

        frame.render_widget(input, area);
    } else {
        let input_line = Line::from(vec![
            Span::raw("> "),
            Span::raw(&display_buffer),
            Span::raw("█"),
        ]);

        let input = Paragraph::new(vec![input_line])
            .block(Block::default().borders(Borders::ALL).title(title));

        frame.render_widget(input, area);
    }
}

fn render_help(frame: &mut Frame, state: &PromptState, area: Rect) {
    let help_text = match state.input_mode {
        InputMode::Line => {
            if state.buffer.is_empty() {
                "Tab: change mode | Enter: submit | Esc: cancel"
            } else {
                "Enter: submit | Esc: cancel"
            }
        }
        InputMode::Multiline => {
            if state.buffer.is_empty() {
                "Tab: change mode | Enter: newline | Ctrl+D: submit | Esc: cancel"
            } else {
                "Enter: newline | Ctrl+D: submit | Esc: cancel"
            }
        }
        InputMode::Hidden => {
            if state.buffer.is_empty() {
                "Tab: change mode | Enter: submit | Esc: cancel"
            } else {
                "Enter: submit | Esc: cancel"
            }
        }
    };

    let help = Paragraph::new(help_text);

    frame.render_widget(help, area);
}
