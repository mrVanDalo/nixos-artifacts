use crate::app::model::{GeneratingState, GenerationStep};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the generation progress view
pub fn render_progress(frame: &mut Frame, state: &GeneratingState, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Header
        Constraint::Length(5), // Progress steps
        Constraint::Min(1),    // Log output
    ])
    .split(area);

    // Header
    render_header(frame, state, chunks[0]);

    // Progress steps
    render_steps(frame, state, chunks[1]);

    // Log output
    render_logs(frame, state, chunks[2]);
}

fn render_header(frame: &mut Frame, state: &GeneratingState, area: Rect) {
    let status_verb = if state.exists {
        "Regenerating"
    } else {
        "Generating"
    };
    let header = Paragraph::new(format!("{} artifact: {}", status_verb, state.artifact_name))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(header, area);
}

fn render_steps(frame: &mut Frame, state: &GeneratingState, area: Rect) {
    let step_style = |is_current: bool, is_done: bool| {
        if is_current {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if is_done {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        }
    };

    let generator_done = state.step == GenerationStep::Serializing;
    let generator_current = state.step == GenerationStep::RunningGenerator;
    let serialize_current = state.step == GenerationStep::Serializing;

    let lines = vec![
        Line::from(vec![
            Span::styled(
                if generator_done {
                    "✓"
                } else if generator_current {
                    "⟳"
                } else {
                    "○"
                },
                step_style(generator_current, generator_done),
            ),
            Span::raw(" "),
            Span::styled(
                "Running generator",
                step_style(generator_current, generator_done),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                if serialize_current { "⟳" } else { "○" },
                step_style(serialize_current, false),
            ),
            Span::raw(" "),
            Span::styled("Serializing files", step_style(serialize_current, false)),
        ]),
    ];

    let steps = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));

    frame.render_widget(steps, area);
}

fn render_logs(frame: &mut Frame, state: &GeneratingState, area: Rect) {
    let lines: Vec<Line> = state
        .log_lines
        .iter()
        .map(|l| Line::from(l.as_str()))
        .collect();

    let logs = Paragraph::new(lines)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL).title("Output"));

    frame.render_widget(logs, area);
}
