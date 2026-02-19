use crate::app::model::ConfirmRegenerateState;
use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Render the regeneration confirmation dialog with side-by-side Leave/Regenerate buttons
pub fn render_confirm_regenerate(frame: &mut Frame, state: &ConfirmRegenerateState, area: Rect) {
    // Clear the background to create modal overlay effect
    frame.render_widget(Clear, area);

    // Create centered dialog area (~60x12)
    let dialog_area = centered_rect(60, 40, area);

    // Render the dialog block with title
    let title = format!("Regenerating artifact: {}", state.artifact_name);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(title)
        .title_style(Style::default().add_modifier(Modifier::BOLD));

    // Get inner area for content
    let inner_area = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    // Build dialog content
    let mut lines = Vec::new();

    // Warning text
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "This will overwrite the existing artifact.",
        Style::default().fg(Color::Yellow),
    )]));

    // Affected targets (if any)
    if !state.affected_targets.is_empty() {
        lines.push(Line::from(""));
        let targets_text = format!("Affected: {}", state.affected_targets.join(", "));
        lines.push(Line::from(vec![Span::styled(
            targets_text,
            Style::default().fg(Color::White),
        )]));
    }

    // Empty line before buttons
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    // Render the text content
    let text_paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(text_paragraph, inner_area);

    // Render buttons in a horizontal layout at the bottom
    let button_area = calculate_button_area(inner_area);
    render_buttons(frame, state, button_area);
}

/// Calculate the area for buttons (bottom portion of dialog)
fn calculate_button_area(inner_area: Rect) -> Rect {
    let button_height = 3u16;
    let vertical_padding = 2u16;

    // Position buttons near bottom with some padding
    let button_y = inner_area
        .height
        .saturating_sub(button_height)
        .saturating_sub(vertical_padding);

    Rect {
        x: inner_area.x,
        y: inner_area.y + button_y,
        width: inner_area.width,
        height: button_height,
    }
}

/// Render the Leave and Regenerate buttons side by side
fn render_buttons(frame: &mut Frame, state: &ConfirmRegenerateState, area: Rect) {
    // Create horizontal layout for two buttons
    let [left_area, right_area] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .flex(Flex::Center)
            .areas(area);

    // Add padding to create space between buttons
    let left_button_area = add_margin(left_area, 1, 0);
    let right_button_area = add_margin(right_area, 1, 0);

    // Render Leave button (left)
    let leave_selected = state.leave_selected;
    render_button(
        frame,
        "Leave",
        leave_selected,
        Color::Green,
        left_button_area,
    );

    // Render Regenerate button (right)
    let regenerate_selected = !state.leave_selected;
    render_button(
        frame,
        "Regenerate",
        regenerate_selected,
        Color::Red,
        right_button_area,
    );
}

/// Render a single button with appropriate styling based on selection
fn render_button(
    frame: &mut Frame,
    label: &str,
    is_selected: bool,
    accent_color: Color,
    area: Rect,
) {
    let (style, border_style) = if is_selected {
        (
            Style::default()
                .bg(accent_color)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(accent_color),
        )
    } else {
        (
            Style::default().fg(Color::Gray),
            Style::default().fg(Color::DarkGray),
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);

    let button_text = if is_selected {
        format!("> {} <", label)
    } else {
        format!("  {}  ", label)
    };

    let paragraph = Paragraph::new(button_text)
        .style(style)
        .alignment(Alignment::Center)
        .block(block);

    frame.render_widget(paragraph, area);
}

/// Add horizontal margin to a rect
fn add_margin(area: Rect, horizontal: u16, _vertical: u16) -> Rect {
    area.inner(Margin::new(horizontal, 0))
}

/// Helper function to create a centered rect within the given area
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);

    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
