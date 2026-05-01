use crate::app::model::{
    ArtifactStatus, GeneratingState, GeneratingSubstate, ListEntry, LogLevel, Step,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
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

/// Render the in-progress view for an artifact in the right pane of the
/// artifact list. Reads the current step from
/// [`ArtifactStatus::Generating`] and live logs from the entry's per-step
/// log buckets — no separate screen state required.
///
/// Caller must ensure `entry.status()` is `Generating(_)`; otherwise a
/// placeholder is rendered.
pub fn render_progress_pane(frame: &mut Frame, entry: &ListEntry, area: Rect) {
    let substate = match entry.status() {
        ArtifactStatus::Generating(substate) => substate,
        _ => {
            let placeholder = Paragraph::new("Not generating").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Generation progress"),
            );
            frame.render_widget(placeholder, area);
            return;
        }
    };

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(1),
    ])
    .split(area);

    let is_regen = entry.runs().len() > 1;
    render_pane_header(frame, entry.artifact_name(), is_regen, chunks[0]);
    render_pane_steps(frame, substate, chunks[1]);
    render_pane_logs(frame, entry, substate.step, chunks[2]);
}

fn render_pane_header(frame: &mut Frame, name: &str, is_regen: bool, area: Rect) {
    let verb = if is_regen {
        "Regenerating"
    } else {
        "Generating"
    };
    let header = Paragraph::new(format!("{}: {}", verb, name))
        .style(Style::default().add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, area);
}

fn render_pane_steps(frame: &mut Frame, substate: &GeneratingSubstate, area: Rect) {
    let line = match substate.step {
        Step::Check => Line::from(vec![
            Span::styled("⟳", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled("Checking", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("  ○ Generate  ○ Serialize"),
        ]),
        Step::Generate => Line::from(vec![
            Span::raw("✓ Check  "),
            Span::styled("⟳", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled("Generating", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("  ○ Serialize"),
        ]),
        Step::Serialize => Line::from(vec![
            Span::raw("✓ Check  ✓ Generate  "),
            Span::styled("⟳", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled("Serializing", Style::default().add_modifier(Modifier::BOLD)),
        ]),
    };

    let steps = Paragraph::new(line).block(Block::default().borders(Borders::NONE));
    frame.render_widget(steps, area);
}

fn render_pane_logs(frame: &mut Frame, entry: &ListEntry, step: Step, area: Rect) {
    let lines: Vec<Line> = entry
        .step_logs()
        .get(step)
        .iter()
        .map(|log_entry| {
            let prefix = match log_entry.level {
                LogLevel::Info => "i",
                LogLevel::Output => "|",
                LogLevel::Error => "!",
                LogLevel::Success => "✓",
            };
            Line::from(vec![
                Span::raw(format!("{} ", prefix)),
                Span::raw(&log_entry.message),
            ])
        })
        .collect();

    let visible_height = area.height.saturating_sub(2) as usize;
    let scroll_offset = if lines.len() > visible_height {
        (lines.len() - visible_height) as u16
    } else {
        0
    };

    let title = format!("{} output", step.label());
    let logs = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));
    frame.render_widget(logs, area);
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
    let generator_done = state.step == Step::Serialize;
    let generator_current = state.step == Step::Generate;
    let serialize_current = state.step == Step::Serialize;

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
