use crate::app::model::SelectGeneratorState;
use crate::config::make::TargetType;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

/// Render the generator selection screen for shared artifacts
pub fn render_generator_selection(frame: &mut Frame, state: &SelectGeneratorState, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();

    for (idx, gen_info) in state.generators.iter().enumerate() {
        let is_selected = idx == state.selected_index;

        // Generator path line with count summary
        let path_style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        // Calculate source counts
        let nixos_count = gen_info
            .sources
            .iter()
            .filter(|s| matches!(s.target_type, TargetType::Nixos))
            .count();
        let home_count = gen_info
            .sources
            .iter()
            .filter(|s| matches!(s.target_type, TargetType::HomeManager))
            .count();
        let count_summary = format_source_counts(nixos_count, home_count);

        // Generator path with usage count
        items.push(ListItem::new(Line::from(vec![
            Span::styled(&gen_info.path, path_style),
            Span::styled(" ", Style::default()),
            Span::styled(
                count_summary,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
        ])));

        // Show which targets use this generator with tree characters and better labels
        let source_count = gen_info.sources.len();
        for (source_idx, source) in gen_info.sources.iter().enumerate() {
            let is_last = source_idx == source_count - 1;
            let tree_char = if is_last { "└─" } else { "├─" };

            let (type_label, type_color) = match source.target_type {
                TargetType::Nixos => ("NixOS", Color::Blue),
                TargetType::HomeManager => ("home-manager", Color::Magenta),
            };

            let indent_style = Style::default().fg(Color::DarkGray);
            items.push(ListItem::new(Line::from(vec![
                Span::styled("    ", indent_style),
                Span::styled(tree_char, indent_style),
                Span::styled(
                    type_label,
                    Style::default().fg(type_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(": ", indent_style),
                Span::styled(&source.target, Style::default().fg(Color::White)),
            ])));
        }

        // Add blank line between generators (except after last one)
        if idx < state.generators.len() - 1 {
            items.push(ListItem::new(Line::from("")));
        }
    }

    let title = format!(
        "Select generator for shared artifact \"{}\"",
        state.artifact_name
    );

    // Build help text showing impact
    let total_sources: usize = state.generators.iter().map(|g| g.sources.len()).sum();
    let help_text = format!(
        " {} generators found, {} total targets. Selected generator will be used for all listed targets. j/k: move, Enter: select, Esc: cancel ",
        state.generators.len(),
        total_sources
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(Line::from(help_text).fg(Color::DarkGray));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    // Calculate the visual index accounting for blank lines and target lists
    let visual_index = calculate_visual_index(state);

    let mut list_state = ListState::default();
    list_state.select(Some(visual_index));

    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Format source counts into a human-readable summary
fn format_source_counts(nixos_count: usize, home_count: usize) -> String {
    let mut parts = Vec::new();

    if nixos_count > 0 {
        let label = if nixos_count == 1 {
            "NixOS machine"
        } else {
            "NixOS machines"
        };
        parts.push(format!("{} {}", nixos_count, label));
    }

    if home_count > 0 {
        let label = if home_count == 1 {
            "home-manager user"
        } else {
            "home-manager users"
        };
        parts.push(format!("{} {}", home_count, label));
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!("({})", parts.join(", "))
    }
}

/// Calculate the visual list index from the logical generator index.
/// This accounts for indented target lines and blank separators.
fn calculate_visual_index(state: &SelectGeneratorState) -> usize {
    let mut visual_idx = 0;
    for i in 0..state.selected_index {
        visual_idx += 1; // Generator path line
        if let Some(generator) = state.generators.get(i) {
            visual_idx += generator.sources.len(); // Target lines
            visual_idx += 1; // Blank separator line
        }
    }
    visual_idx
}
