use crate::app::model::SelectGeneratorState;
use crate::config::make::TargetType;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }
    let prefix_len = max_len / 3;
    let suffix_len = max_len / 3;
    let ellipsis = "...";
    format!(
        "{}{}{}",
        &path[..prefix_len],
        ellipsis,
        &path[path.len() - suffix_len..]
    )
}

fn format_targets_with_prefix(targets: &[String], prefix: &str, max_display: usize) -> Vec<String> {
    let mut sorted = targets.to_vec();
    sorted.sort();

    let mut result = Vec::new();
    let display_count = sorted.len().min(max_display);

    for target in &sorted[..display_count] {
        result.push(format!("{}: {}", prefix, target));
    }

    if sorted.len() > max_display {
        result.push(format!("+{} more", sorted.len() - max_display));
    }

    result
}

fn format_all_targets(
    nixos_targets: &[String],
    home_targets: &[String],
    max_display: usize,
) -> Vec<String> {
    let mut lines = Vec::new();

    lines.extend(format_targets_with_prefix(
        nixos_targets,
        "nixos",
        max_display,
    ));

    lines.extend(format_targets_with_prefix(
        home_targets,
        "home",
        max_display,
    ));

    lines
}

fn separator_line(width: usize) -> Span<'static> {
    Span::raw("─".repeat(width))
}

#[allow(clippy::too_many_lines)]
pub fn render_generator_selection(frame: &mut Frame, state: &SelectGeneratorState, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();
    let block_inner_width = area.width.saturating_sub(2) as usize;

    let type_indicator = if state.nixos_targets.len() + state.home_targets.len() > 1 {
        "Shared artifact"
    } else {
        "Per-machine artifact"
    };
    items.push(ListItem::new(Line::from(type_indicator)));

    items.push(ListItem::new(Line::from(vec![separator_line(
        block_inner_width,
    )])));

    let title_text = format!("Select generator for {}", state.artifact_name);
    items.push(ListItem::new(Line::from(Span::styled(
        title_text,
        Style::default().add_modifier(Modifier::BOLD),
    ))));

    items.push(ListItem::new(Line::from(vec![separator_line(
        block_inner_width,
    )])));

    let description_text = state
        .description
        .as_deref()
        .unwrap_or("No description provided");
    items.push(ListItem::new(Line::from(description_text)));

    items.push(ListItem::new(Line::from(vec![separator_line(
        block_inner_width,
    )])));

    if !state.prompts.is_empty() {
        for (idx, prompt) in state.prompts.iter().enumerate() {
            let prompt_line = match &prompt.description {
                Some(desc) => format!("{}. {}: {}", idx + 1, prompt.name, desc),
                None => format!("{}. {}", idx + 1, prompt.name),
            };
            items.push(ListItem::new(Line::from(Span::raw(prompt_line))));
        }
        items.push(ListItem::new(Line::from(vec![separator_line(
            block_inner_width,
        )])));
    }

    for (idx, gen_info) in state.generators.iter().enumerate() {
        let is_selected = idx == state.selected_index;

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

        let count_str = format_count_summary(nixos_count, home_count);
        let available_width = block_inner_width.saturating_sub(4 + count_str.len());
        let display_path = truncate_path(&gen_info.path, available_width);

        let indicator = if is_selected { "> " } else { "  " };

        let path_style = if is_selected {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        items.push(ListItem::new(Line::from(vec![
            Span::raw(indicator),
            Span::styled(display_path.clone(), path_style),
            Span::styled(
                format!("  {}", count_str),
                Style::default().add_modifier(Modifier::ITALIC),
            ),
        ])));
    }

    items.push(ListItem::new(Line::from(vec![separator_line(
        block_inner_width,
    )])));

    items.push(ListItem::new(Line::from(Span::styled(
        "All targets:",
        Style::default().add_modifier(Modifier::BOLD),
    ))));

    let target_lines = format_all_targets(&state.nixos_targets, &state.home_targets, 10);
    if target_lines.is_empty() {
        items.push(ListItem::new(Line::from(Span::raw("  (none)"))));
    } else {
        for line in target_lines {
            items.push(ListItem::new(Line::from(Span::raw(line))));
        }
    }

    items.push(ListItem::new(Line::from(vec![separator_line(
        block_inner_width,
    )])));

    let help_text = "j/k: move, Enter: select, Esc: cancel";
    items.push(ListItem::new(Line::from(Span::raw(help_text))));

    let title = format!("Artifact: {}", state.artifact_name);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().add_modifier(Modifier::BOLD));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    let visual_index = calculate_visual_index(state, block_inner_width);

    let mut list_state = ListState::default();
    list_state.select(Some(visual_index));

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn format_count_summary(nixos_count: usize, home_count: usize) -> String {
    let mut parts = Vec::new();

    if nixos_count > 0 {
        let label = if nixos_count == 1 {
            "machine"
        } else {
            "machines"
        };
        parts.push(format!("{} {}", nixos_count, label));
    }

    if home_count > 0 {
        let label = if home_count == 1 { "user" } else { "users" };
        parts.push(format!("{} {}", home_count, label));
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!("({})", parts.join(", "))
    }
}

fn calculate_visual_index(state: &SelectGeneratorState, _width: usize) -> usize {
    let mut idx = 0;

    idx += 6;

    if !state.prompts.is_empty() {
        idx += state.prompts.len();
        idx += 1;
    }

    idx += state.selected_index;

    idx
}
