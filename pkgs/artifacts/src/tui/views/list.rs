use crate::app::model::{ArtifactStatus, Model, TargetType};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

/// Render the artifact list view
pub fn render_artifact_list(frame: &mut Frame, model: &Model, area: Rect) {
    // Split area: list takes most space, legend gets 1 line at bottom
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
        "Artifacts ({}) - j/k: move, Enter: generate, q: quit",
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

    // Render legend
    let legend = Line::from(vec![
        Span::styled("○", Style::default().fg(Color::Gray)),
        Span::raw(" Pending  "),
        Span::styled("◐", Style::default().fg(Color::Yellow)),
        Span::raw(" Needs Gen  "),
        Span::styled("✓", Style::default().fg(Color::Green)),
        Span::raw(" Up To Date  "),
        Span::styled("✗", Style::default().fg(Color::Red)),
        Span::raw(" Failed  "),
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled("[N]", Style::default().fg(Color::DarkGray)),
        Span::raw(" NixOS  "),
        Span::styled("[H]", Style::default().fg(Color::DarkGray)),
        Span::raw(" Home"),
    ]);
    frame.render_widget(Paragraph::new(legend), legend_area);
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
