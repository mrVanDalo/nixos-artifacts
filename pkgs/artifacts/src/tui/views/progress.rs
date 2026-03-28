use crate::app::model::{GeneratingState, GenerationStep};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn render_progress(frame: &mut Frame, state: &GeneratingState, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(5),
        Constraint::Min(1),
    ])
    .split(area);

    render_header(frame, state, chunks[0]);
    render_steps(frame, state, chunks[1]);
    render_logs(frame, state, chunks[2]);
}

fn render_header(frame: &mut Frame, state: &GeneratingState, area: Rect) {
    let status_verb = if state.exists {
        "Regenerating"
    } else {
        "Generating"
    };
    let header = Paragraph::new(format!("{} artifact: {}", status_verb, state.artifact_name))
        .style(Style::default().add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(header, area);
}

fn render_steps(frame: &mut Frame, state: &GeneratingState, area: Rect) {
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
                Style::default().add_modifier(if generator_current {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ),
            Span::raw(" "),
            Span::styled(
                "Running generator",
                Style::default().add_modifier(if generator_current || generator_done {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                if serialize_current { "⟳" } else { "○" },
                Style::default().add_modifier(if serialize_current {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ),
            Span::raw(" "),
            Span::styled(
                "Serializing files",
                Style::default().add_modifier(if serialize_current {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ),
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

    let logs = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Output"));

    frame.render_widget(logs, area);
}
