use crate::app::model::{ArtifactStatus, ListEntry, LogLevel, LogStep, Model, TargetType};
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
        .entries
        .iter()
        .map(|entry| {
            let (icon, style, status_text) = status_display_with_text(entry);

            // Render based on entry type
            let content = match entry {
                ListEntry::Single(single) => {
                    let target_type_icon = match &single.target_type {
                        TargetType::NixOS { .. } => "N",
                        TargetType::HomeManager { .. } => "H",
                        TargetType::Shared { .. } => "S",
                    };
                    let target_name = single.target_type.target_name().unwrap_or("unknown");
                    let mut spans = vec![
                        Span::styled(icon, style),
                        Span::raw(" "),
                        Span::styled(
                            format!("[{}]", target_type_icon),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::raw(" "),
                        Span::raw(target_name),
                        Span::styled("/", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            &single.artifact.name,
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ];
                    // Add status text for generating state
                    if let Some(text) = status_text {
                        spans.push(Span::raw(" "));
                        spans.push(Span::styled(text, style));
                    }
                    Line::from(spans)
                }
                ListEntry::Shared(shared) => {
                    let target_count =
                        shared.info.nixos_targets.len() + shared.info.home_targets.len();
                    let mut spans = vec![
                        Span::styled(icon, style),
                        Span::raw(" "),
                        Span::styled("[S]", Style::default().fg(Color::DarkGray)),
                        Span::raw(" "),
                        Span::styled(
                            &shared.info.artifact_name,
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("({} targets)", target_count),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ];
                    // Add status text for generating state
                    if let Some(text) = status_text {
                        spans.push(Span::raw(" "));
                        spans.push(Span::styled(text, style));
                    }
                    Line::from(spans)
                }
            };
            ListItem::new(content)
        })
        .collect();

    let title = format!(
        "Artifacts ({}) - j/k: move, Enter: gen, l: logs, q: quit",
        model.entries.len()
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
    let selected_entry = model.entries.get(model.selected_index);

    let title = match selected_entry {
        Some(ListEntry::Single(entry)) => format!(
            "Logs: {}/{}",
            entry.target_type.target_name().unwrap_or("unknown"),
            entry.artifact.name
        ),
        Some(ListEntry::Shared(entry)) => format!("Logs: {}", entry.info.artifact_name),
        None => "Logs".to_string(),
    };

    let mut lines: Vec<Line> = Vec::new();

    // Show error details if the artifact has failed status
    #[allow(clippy::collapsible_if)]
    if let Some(entry) = selected_entry {
        if let ArtifactStatus::Failed {
            error,
            output,
            retry_available,
        } = entry.status()
        {
            // Error header - distinguish between config errors and runtime failures
            if *retry_available {
                // Runtime failure - can retry
                lines.push(Line::from(vec![
                    Span::styled(
                        "✗ ",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "FAILED",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else {
                // Configuration error - cannot retry
                lines.push(Line::from(vec![
                    Span::styled(
                        "⚠ ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "CONFIGURATION ERROR",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
            lines.push(Line::from(vec![
                Span::styled(
                    "Error: ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(error, Style::default().fg(Color::Red)),
            ]));

            // Show output if available
            if !output.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "Output:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )]));
                for line in output.lines() {
                    if !line.is_empty() {
                        lines.push(Line::from(vec![
                            Span::styled("  ", Style::default()),
                            Span::styled(line, Style::default().fg(Color::DarkGray)),
                        ]));
                    }
                }
            }

            // Separator between error and logs
            lines.push(Line::from(Span::styled(
                "─".repeat(area.width as usize),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    // Determine which steps have logs (only show steps that have happened)
    let visible_steps: Vec<LogStep> = match selected_entry {
        Some(entry) => [LogStep::Check, LogStep::Generate, LogStep::Serialize]
            .into_iter()
            .filter(|step| !entry.step_logs().get(*step).is_empty())
            .collect(),
        None => vec![],
    };

    if visible_steps.is_empty() && lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No logs yet. Generate this artifact to see output.",
            Style::default().fg(Color::DarkGray),
        )));
    } else if !visible_steps.is_empty() {
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
            if is_selected && let Some(entry) = selected_entry {
                for log_entry in entry.step_logs().get(*step) {
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

/// Status display that also returns text for generating state
/// Returns (icon, style, optional_status_text)
fn status_display_with_text(entry: &ListEntry) -> (&'static str, Style, Option<String>) {
    let status = entry.status();
    let icon_style = match status {
        ArtifactStatus::Pending => ("○", Style::default().fg(Color::Gray)),
        ArtifactStatus::NeedsGeneration => ("◐", Style::default().fg(Color::Yellow)),
        ArtifactStatus::UpToDate => ("✓", Style::default().fg(Color::Green)),
        ArtifactStatus::Generating(_) => ("⟳", Style::default().fg(Color::Cyan)),
        ArtifactStatus::Failed { .. } => ("✗", Style::default().fg(Color::Red)),
    };

    // Note: When an artifact is in Generating state, we're typically showing
    // the Generating screen, not the list. If we somehow show the list with
    // Generating status (edge case), default to "Generating..." since we
    // can't know if it existed before the check ran.
    let status_text = if matches!(status, ArtifactStatus::Generating(_)) {
        Some("Generating...".to_string())
    } else {
        None
    };

    (icon_style.0, icon_style.1, status_text)
}
