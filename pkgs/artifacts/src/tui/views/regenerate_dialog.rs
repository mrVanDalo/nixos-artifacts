use crate::app::model::ConfirmRegenerateState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render_confirm_regenerate(frame: &mut Frame, state: &ConfirmRegenerateState, area: Rect) {
    frame.render_widget(Clear, area);

    let dialog_area = centered_rect(60, 40, area);

    let title = format!("Regenerating artifact: {}", state.artifact_name);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().add_modifier(Modifier::BOLD));

    let inner_area = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let mut lines = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::raw(
        "This will overwrite the existing artifact.",
    )]));

    if !state.affected_targets.is_empty() {
        lines.push(Line::from(""));
        let targets_text = format!("Affected: {}", state.affected_targets.join(", "));
        lines.push(Line::from(vec![Span::raw(targets_text)]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(""));

    let text_paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(text_paragraph, inner_area);

    let button_area = calculate_button_area(inner_area);
    render_buttons(frame, state, button_area);
}

fn calculate_button_area(inner_area: Rect) -> Rect {
    let button_height = 3u16;
    let vertical_padding = 2u16;

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

fn render_buttons(frame: &mut Frame, state: &ConfirmRegenerateState, area: Rect) {
    let [left_area, right_area] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .flex(Flex::Center)
            .areas(area);

    let left_button_area = add_margin(left_area, 1, 0);
    let right_button_area = add_margin(right_area, 1, 0);

    let leave_selected = state.leave_selected;
    render_button(
        frame,
        "Leave",
        leave_selected,
        left_button_area,
    );

    let regenerate_selected = !state.leave_selected;
    render_button(
        frame,
        "Regenerate",
        regenerate_selected,
        right_button_area,
    );
}

fn render_button(
    frame: &mut Frame,
    label: &str,
    is_selected: bool,
    area: Rect,
) {
    let (style, border_style) = if is_selected {
        (
            Style::default().add_modifier(Modifier::BOLD),
            Style::default().add_modifier(Modifier::BOLD),
        )
    } else {
        (Style::default(), Style::default())
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

fn add_margin(area: Rect, horizontal: u16, _vertical: u16) -> Rect {
    area.inner(Margin::new(horizontal, 0))
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);

    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}