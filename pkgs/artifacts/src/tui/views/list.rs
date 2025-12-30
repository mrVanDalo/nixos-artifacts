use crate::app::model::{ArtifactStatus, LogLevel, LogStep, Model, TargetType};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

/// Render the artifact list view with log panel
pub fn render_artifact_list(frame: &mut Frame, model: &Model, area: Rect) {
    // Main horizontal split: artifact list (left) | log panel (right)
    let horizontal_chunks =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);

    let list_area = horizontal_chunks[0];
    let log_area = horizontal_chunks[1];

    // Left panel: artifact list with legend
    render_artifact_list_panel(frame, model, list_area);

    // Right panel: logs for selected artifact
    render_log_panel(frame, model, log_area);
}

fn render_artifact_list_panel(frame: &mut Frame, model: &Model, area: Rect) {
    // Split: list takes most space, legend gets 1 line at bottom
    let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(area);
    let list_area = chunks[0];
    let legend_area = chunks[1];

    let items: Vec<ListItem> = model
        .artifacts
        .iter()
        .map(|entry| {
            let (icon, style) = status_display(&entry.status);
            let target_type_icon = match entry.target_type {
                TargetType::Nixos => "N",
                TargetType::HomeManager => "H",
            };
            let content = Line::from(vec![
                Span::styled(icon, style),
                Span::raw(" "),
                Span::styled(
                    format!("[{}]", target_type_icon),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(" "),
                Span::raw(&entry.target),
                Span::styled("/", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    &entry.artifact.name,
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]);
            ListItem::new(content)
        })
        .collect();

    let title = format!(
        "Artifacts ({}) - j/k: move, Tab: logs, Enter: gen, q: quit",
        model.artifacts.len()
    );

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(Some(model.selected_index));

    frame.render_stateful_widget(list, list_area, &mut state);

    // Render legend (compressed for narrower space)
    let legend = Line::from(vec![
        Span::styled("○", Style::default().fg(Color::Gray)),
        Span::raw(" Pend "),
        Span::styled("◐", Style::default().fg(Color::Yellow)),
        Span::raw(" Need "),
        Span::styled("✓", Style::default().fg(Color::Green)),
        Span::raw(" OK "),
        Span::styled("✗", Style::default().fg(Color::Red)),
        Span::raw(" Fail"),
    ]);
    frame.render_widget(Paragraph::new(legend), legend_area);
}

fn render_log_panel(frame: &mut Frame, model: &Model, area: Rect) {
    let selected_artifact = model.artifacts.get(model.selected_index);

    let title = match selected_artifact {
        Some(entry) => format!("Logs: {}/{}", entry.target, entry.artifact.name),
        None => "Logs".to_string(),
    };

    let mut lines: Vec<Line> = Vec::new();

    // Determine which steps have logs (only show steps that have happened)
    let visible_steps: Vec<LogStep> = match selected_artifact {
        Some(entry) => [LogStep::Check, LogStep::Generate, LogStep::Serialize]
            .into_iter()
            .filter(|step| !entry.step_logs.get(*step).is_empty())
            .collect(),
        None => vec![],
    };

    if visible_steps.is_empty() {
        lines.push(Line::from(Span::styled(
            "No logs yet. Generate this artifact to see output.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        // Render accordion-style steps (only those with logs)
        for step in &visible_steps {
            let is_selected = *step == model.selected_log_step;
            let icon = if is_selected { "▼" } else { "▶" };

            // Step header line
            lines.push(Line::from(vec![
                Span::styled(format!("{} ", icon), Style::default().fg(Color::Cyan)),
                Span::styled(step.label(), Style::default().add_modifier(Modifier::BOLD)),
            ]));

            // If expanded, show logs indented
            if is_selected && let Some(entry) = selected_artifact {
                for log_entry in entry.step_logs.get(*step) {
                    let (prefix, style) = match log_entry.level {
                        LogLevel::Info => ("i", Style::default().fg(Color::Blue)),
                        LogLevel::Output => ("|", Style::default().fg(Color::White)),
                        LogLevel::Error => ("!", Style::default().fg(Color::Red)),
                        LogLevel::Success => ("✓", Style::default().fg(Color::Green)),
                    };

                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", prefix), style),
                        Span::styled(&log_entry.message, style),
                    ]));
                }
            }
        }
    }

    // Calculate scroll offset to show latest logs in expanded section
    let visible_height = area.height.saturating_sub(2) as usize; // Subtract border
    let scroll_offset = if lines.len() > visible_height {
        (lines.len() - visible_height) as u16
    } else {
        0
    };

    let log_paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));

    frame.render_widget(log_paragraph, area);
}

fn status_display(status: &ArtifactStatus) -> (&'static str, Style) {
    match status {
        ArtifactStatus::Pending => ("○", Style::default().fg(Color::Gray)),
        ArtifactStatus::NeedsGeneration => ("◐", Style::default().fg(Color::Yellow)),
        ArtifactStatus::UpToDate => ("✓", Style::default().fg(Color::Green)),
        ArtifactStatus::Generating => ("⟳", Style::default().fg(Color::Cyan)),
        ArtifactStatus::Failed(_) => ("✗", Style::default().fg(Color::Red)),
    }
}
