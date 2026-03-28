use crate::app::model::{ArtifactStatus, ListEntry, LogLevel, LogStep, Model, TargetType};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

pub fn render_artifact_list(frame: &mut Frame, model: &Model, area: Rect) {
    let horizontal_chunks =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);

    let list_area = horizontal_chunks[0];
    let log_area = horizontal_chunks[1];

    render_artifact_list_panel(frame, model, list_area);
    render_log_panel(frame, model, log_area);
}

fn render_artifact_list_panel(frame: &mut Frame, model: &Model, area: Rect) {
    let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(area);
    let list_area = chunks[0];
    let legend_area = chunks[1];

    let items: Vec<ListItem> = model
        .entries
        .iter()
        .map(|entry| {
            let (icon, style, status_text) = status_display_with_text(entry);

            let content = match entry {
                ListEntry::Single(single) => {
                    let target_type_icon = match &single.target_type {
                        TargetType::NixOS { .. } => "N",
                        TargetType::HomeManager { .. } => "H",
                    };
                    let target_name = single.target_type.target_name();
                    let mut spans = vec![
                        Span::styled(icon, style),
                        Span::raw(" "),
                        Span::raw(format!("[{}]", target_type_icon)),
                        Span::raw(" "),
                        Span::raw(target_name),
                        Span::raw("/"),
                        Span::styled(
                            &single.artifact.name,
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ];
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
                        Span::raw("[S]"),
                        Span::raw(" "),
                        Span::styled(
                            &shared.info.artifact_name,
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::raw(format!("({} targets)", target_count)),
                    ];
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
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(Some(model.selected_index));

    frame.render_stateful_widget(list, list_area, &mut state);

    let legend = Line::from(vec![
        Span::raw("○ Pend "),
        Span::raw("◐ Need "),
        Span::raw("✓ OK "),
        Span::raw("✗ Fail"),
    ]);
    frame.render_widget(Paragraph::new(legend), legend_area);
}

fn render_log_panel(frame: &mut Frame, model: &Model, area: Rect) {
    let selected_entry = model.entries.get(model.selected_index);

    let title = match selected_entry {
        Some(ListEntry::Single(entry)) => format!(
            "Logs: {}/{}",
            entry.target_type.target_name(),
            entry.artifact.name
        ),
        Some(ListEntry::Shared(entry)) => format!("Logs: {}", entry.info.artifact_name),
        None => "Logs".to_string(),
    };

    let mut lines: Vec<Line> = Vec::new();

    #[allow(clippy::collapsible_if)]
    if let Some(entry) = selected_entry {
        if let ArtifactStatus::Failed {
            error,
            output,
            retry_available,
        } = entry.status()
        {
            if *retry_available {
                lines.push(Line::from(vec![
                    Span::styled("✗ ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled("FAILED", Style::default().add_modifier(Modifier::BOLD)),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("⚠ ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled("CONFIGURATION ERROR", Style::default().add_modifier(Modifier::BOLD)),
                ]));
            }
            lines.push(Line::from(vec![
                Span::styled("Error: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(error),
            ]));

            if !output.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    "Output:",
                    Style::default().add_modifier(Modifier::BOLD),
                )]));
                for line in output.lines() {
                    if !line.is_empty() {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::raw(line),
                        ]));
                    }
                }
            }

            lines.push(Line::from(Span::raw(
                "─".repeat(area.width as usize),
            )));
        }
    }

    let visible_steps: Vec<LogStep> = match selected_entry {
        Some(entry) => [LogStep::Check, LogStep::Generate, LogStep::Serialize]
            .into_iter()
            .filter(|step| !entry.step_logs().get(*step).is_empty())
            .collect(),
        None => vec![],
    };

    if visible_steps.is_empty() && lines.is_empty() {
        lines.push(Line::from(Span::raw(
            "No logs yet. Generate this artifact to see output.",
        )));
    } else if !visible_steps.is_empty() {
        for step in &visible_steps {
            let is_selected = *step == model.selected_log_step;
            let icon = if is_selected { "▼" } else { "▶" };

            lines.push(Line::from(vec![
                Span::raw(format!("{} ", icon)),
                Span::styled(step.label(), Style::default().add_modifier(Modifier::BOLD)),
            ]));

            if is_selected && let Some(entry) = selected_entry {
                for log_entry in entry.step_logs().get(*step) {
                    let prefix = match log_entry.level {
                        LogLevel::Info => "i",
                        LogLevel::Output => "|",
                        LogLevel::Error => "!",
                        LogLevel::Success => "✓",
                    };

                    lines.push(Line::from(vec![
                        Span::raw(format!("  {} ", prefix)),
                        Span::raw(&log_entry.message),
                    ]));
                }
            }
        }
    }

    let visible_height = area.height.saturating_sub(2) as usize;
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

fn status_display_with_text(entry: &ListEntry) -> (&'static str, Style, Option<String>) {
    let status = entry.status();
    let icon = match status {
        ArtifactStatus::Pending => "○",
        ArtifactStatus::NeedsGeneration => "◐",
        ArtifactStatus::UpToDate => "✓",
        ArtifactStatus::Generating(_) => "⟳",
        ArtifactStatus::Failed { .. } => "✗",
    };

    let status_text = if matches!(status, ArtifactStatus::Generating(_)) {
        Some("Generating...".to_string())
    } else {
        None
    };

    (icon, Style::default(), status_text)
}