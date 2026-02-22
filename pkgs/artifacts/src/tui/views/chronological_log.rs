use crate::app::model::{ChronologicalLogState, LogStep, StepLogs};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the chronological log view with expandable sections
pub fn render_chronological_log(
    frame: &mut Frame,
    model: &crate::app::model::Model,
    state: &ChronologicalLogState,
    area: Rect,
) {
    // Split area into header and content
    let chunks = Layout::vertical([
        Constraint::Length(3), // Header with artifact name
        Constraint::Min(0),    // Scrollable log content
    ])
    .split(area);

    // Render header
    render_header(frame, state, chunks[0]);

    // Render log sections
    render_log_sections(frame, model, state, chunks[1]);
}

/// Render the header showing artifact name and navigation hints
fn render_header(frame: &mut Frame, state: &ChronologicalLogState, area: Rect) {
    let header_text = vec![Line::from(vec![
        Span::styled(
            format!("Artifact: {} ", state.artifact_name),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "(Press 'q' to return, 'e' to expand all, 'c' to collapse all)",
            Style::default().fg(Color::DarkGray),
        ),
    ])];

    let header = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(header, area);
}

/// Render the log sections (Check, Generate, Serialize)
fn render_log_sections(
    frame: &mut Frame,
    model: &crate::app::model::Model,
    state: &ChronologicalLogState,
    area: Rect,
) {
    // Get the artifact's step logs
    let step_logs = if let Some(entry) = model.entries.get(state.artifact_index) {
        entry.step_logs()
    } else {
        return;
    };

    // Split area into content and legend
    let chunks = Layout::vertical([
        Constraint::Min(0),    // Scrollable log content
        Constraint::Length(1), // Legend/help line
    ])
    .split(area);

    // Render the scrollable content area
    render_scrollable_content(frame, state, step_logs, chunks[0]);

    // Render the legend
    render_legend(frame, chunks[1]);
}

/// Render scrollable content area
fn render_scrollable_content(
    frame: &mut Frame,
    state: &ChronologicalLogState,
    step_logs: &StepLogs,
    area: Rect,
) {
    // Calculate layout for three sections
    let sections = [LogStep::Check, LogStep::Generate, LogStep::Serialize];

    // Each collapsed section gets fixed height, expanded sections share remaining space
    let constraints: Vec<Constraint> = sections
        .iter()
        .map(|step| {
            if state.is_expanded(*step) {
                // Expanded sections get proportional space
                Constraint::Min(3)
            } else {
                // Collapsed sections get minimal space for header
                Constraint::Length(1)
            }
        })
        .collect();

    let chunks = Layout::vertical(constraints).split(area);

    for (idx, step) in sections.iter().enumerate() {
        render_section(frame, state, step, step_logs, chunks[idx]);
    }
}

/// Render the keybinding legend at the bottom
fn render_legend(frame: &mut Frame, area: Rect) {
    let legend_text = Line::from(vec![
        Span::styled(
            "Space/Enter",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Toggle  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "+/-",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Expand/Collapse  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "j/k",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Navigate  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "PgUp/PgDn",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Scroll  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "Esc/q",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(": Back", Style::default().fg(Color::DarkGray)),
    ]);

    let legend = Paragraph::new(legend_text).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(legend, area);
}

/// Calculate summary text for section header
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

/// Render a single log section
fn render_section(
    frame: &mut Frame,
    state: &ChronologicalLogState,
    step: &LogStep,
    step_logs: &StepLogs,
    area: Rect,
) {
    let is_expanded = state.is_expanded(*step);
    let is_focused = state.focused_section == Some(*step);

    // Get logs for this section
    let logs = step_logs.get(*step);

    // Build section header
    let expand_icon = if is_expanded { "▼" } else { "▶" };
    let step_name = step.label();
    let summary = calculate_summary(logs);
    let focus_indicator = if is_focused { "→ " } else { "  " };

    let header_text = Line::from(vec![
        Span::styled(focus_indicator, Style::default().fg(Color::Cyan)),
        Span::styled(expand_icon.to_string(), Style::default().fg(Color::Yellow)),
        Span::styled(" ", Style::default()),
        Span::styled(
            step_name.to_string(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" ({})", summary),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    if !is_expanded {
        // Render just the header line for collapsed sections
        let header = Paragraph::new(vec![header_text]);
        frame.render_widget(header, area);
    } else {
        // Render expanded section with borders
        let border_style = if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Convert log entries to lines
        let log_lines: Vec<Line> = logs
            .iter()
            .map(|log| {
                let (prefix, style) = match log.level {
                    crate::app::model::LogLevel::Info => {
                        ("[INFO] ", Style::default().fg(Color::Blue))
                    }
                    crate::app::model::LogLevel::Output => ("", Style::default().fg(Color::White)),
                    crate::app::model::LogLevel::Error => {
                        ("[ERROR] ", Style::default().fg(Color::Red))
                    }
                    crate::app::model::LogLevel::Success => {
                        ("[OK] ", Style::default().fg(Color::Green))
                    }
                };
                Line::from(vec![
                    Span::styled(prefix.to_string(), style),
                    Span::styled(log.message.clone(), style),
                ])
            })
            .collect();

        let content = Paragraph::new(log_lines)
            .block(
                Block::default()
                    .title(header_text)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .scroll((state.scroll_offset as u16, 0));

        frame.render_widget(content, area);
    }
}
