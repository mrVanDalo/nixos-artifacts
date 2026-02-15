use crate::app::model::SelectGeneratorState;
use crate::config::make::TargetType;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

/// Render the generator selection screen for shared artifacts
pub fn render_generator_selection(frame: &mut Frame, state: &SelectGeneratorState, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();

    for (idx, gen_info) in state.generators.iter().enumerate() {
        let is_selected = idx == state.selected_index;

        // Generator path line
        let path_style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        items.push(ListItem::new(Line::from(vec![Span::styled(
            &gen_info.path,
            path_style,
        )])));

        // Show which targets use this generator (indented)
        for source in &gen_info.sources {
            let target_type_label = match source.target_type {
                TargetType::Nixos => "nixos",
                TargetType::HomeManager => "homemanager",
            };
            let indent_style = Style::default().fg(Color::DarkGray);
            items.push(ListItem::new(Line::from(vec![
                Span::styled("    - ", indent_style),
                Span::styled(&source.target, Style::default().fg(Color::Gray)),
                Span::styled(" (", indent_style),
                Span::styled(target_type_label, indent_style),
                Span::styled(")", indent_style),
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

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_bottom(Line::from(" j/k: move, Enter: select, Esc: cancel ").fg(Color::DarkGray));

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

    // Help text at bottom
    let help = Paragraph::new(Line::from(vec![Span::styled(
        "Multiple generators found. Select one to use for generation.",
        Style::default().fg(Color::Yellow),
    )]));

    // This would go below the list but for simplicity we're including hints in the block title
    let _ = help;
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
