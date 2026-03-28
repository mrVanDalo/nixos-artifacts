use crate::app::model::{ChronologicalLogState, LogStep, StepLogs};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn render_chronological_log(
    frame: &mut Frame,
    model: &crate::app::model::Model,
    state: &ChronologicalLogState,
    area: Rect,
) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);

    render_header(frame, state, chunks[0]);
    render_log_sections(frame, model, state, chunks[1]);
}

fn render_header(frame: &mut Frame, state: &ChronologicalLogState, area: Rect) {
    let header_text = vec![Line::from(vec![
        Span::styled(
            format!("Artifact: {} ", state.artifact_name),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("(Press 'q' to return, 'e' to expand all, 'c' to collapse all)"),
    ])];

    let header = Paragraph::new(header_text).block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, area);
}

fn render_log_sections(
    frame: &mut Frame,
    model: &crate::app::model::Model,
    state: &ChronologicalLogState,
    area: Rect,
) {
    let step_logs = if let Some(entry) = model.entries.get(state.artifact_index) {
        entry.step_logs()
    } else {
        return;
    };

    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    render_scrollable_content(frame, state, step_logs, chunks[0]);
    render_legend(frame, chunks[1]);
}

fn render_scrollable_content(
    frame: &mut Frame,
    state: &ChronologicalLogState,
    step_logs: &StepLogs,
    area: Rect,
) {
    let sections = [LogStep::Check, LogStep::Generate, LogStep::Serialize];

    let constraints: Vec<Constraint> = sections
        .iter()
        .map(|step| {
            if state.is_expanded(*step) {
                Constraint::Min(3)
            } else {
                Constraint::Length(1)
            }
        })
        .collect();

    let chunks = Layout::vertical(constraints).split(area);

    for (idx, step) in sections.iter().enumerate() {
        render_section(frame, state, step, step_logs, chunks[idx]);
    }
}

fn render_legend(frame: &mut Frame, area: Rect) {
    let legend_text = Line::from(vec![
        Span::styled("Space/Enter", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Toggle  "),
        Span::styled("+/-", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Expand/Collapse  "),
        Span::styled("j/k", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Navigate  "),
        Span::styled("PgUp/PgDn", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Scroll  "),
        Span::styled("Esc/q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Back"),
    ]);

    let legend = Paragraph::new(legend_text).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(legend, area);
}

fn calculate_summary(logs: &[crate::app::model::LogEntry]) -> String {
    let error_count = logs
        .iter()
        .filter(|l| matches!(l.level, crate::app::model::LogLevel::Error))
        .count();
    let success_count = logs
        .iter()
        .filter(|l| matches!(l.level, crate::app::model::LogLevel::Success))
        .count();

    if error_count > 0 {
        format!("{} lines, {} errors", logs.len(), error_count)
    } else if success_count > 0 {
        format!("{} lines, {} success", logs.len(), success_count)
    } else {
        format!("{} lines", logs.len())
    }
}

fn render_section(
    frame: &mut Frame,
    state: &ChronologicalLogState,
    step: &LogStep,
    step_logs: &StepLogs,
    area: Rect,
) {
    let is_expanded = state.is_expanded(*step);
    let is_focused = state.focused_section == Some(*step);

    let logs = step_logs.get(*step);

    let expand_icon = if is_expanded { "▼" } else { "▶" };
    let step_name = step.label();
    let summary = calculate_summary(logs);
    let focus_indicator = if is_focused { "→ " } else { "  " };

    let header_text = Line::from(vec![
        Span::raw(focus_indicator),
        Span::raw(expand_icon.to_string()),
        Span::raw(" "),
        Span::styled(
            step_name.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" ({})", summary)),
    ]);

    if !is_expanded {
        let header = Paragraph::new(vec![header_text]);
        frame.render_widget(header, area);
    } else {
        let log_lines: Vec<Line> = logs
            .iter()
            .map(|log| {
                let prefix = match log.level {
                    crate::app::model::LogLevel::Info => "[INFO] ",
                    crate::app::model::LogLevel::Output => "",
                    crate::app::model::LogLevel::Error => "[ERROR] ",
                    crate::app::model::LogLevel::Success => "[OK] ",
                };
                Line::from(vec![
                    Span::raw(prefix.to_string()),
                    Span::raw(log.message.clone()),
                ])
            })
            .collect();

        let content = Paragraph::new(log_lines)
            .block(
                Block::default()
                    .title(header_text)
                    .borders(Borders::ALL)
                    .border_style(Style::default()),
            )
            .scroll((state.scroll_offset as u16, 0));

        frame.render_widget(content, area);
    }
}
