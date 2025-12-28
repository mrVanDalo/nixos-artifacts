mod list;
mod prompt;
mod progress;

use crate::app::model::{Model, Screen};
use ratatui::Frame;

pub use list::render_artifact_list;
pub use prompt::render_prompt;
pub use progress::render_progress;

/// Top-level view dispatcher - renders the appropriate screen based on model state
pub fn render(frame: &mut Frame, model: &Model) {
    let area = frame.area();

    match &model.screen {
        Screen::ArtifactList => render_artifact_list(frame, model, area),
        Screen::Prompt(state) => render_prompt(frame, state, area),
        Screen::Generating(state) => render_progress(frame, state, area),
        Screen::Done(state) => render_done(frame, state, area),
    }

    // Render error popup if present
    if let Some(ref error) = model.error {
        render_error_popup(frame, error);
    }
}

fn render_done(frame: &mut Frame, state: &crate::app::model::DoneState, area: ratatui::layout::Rect) {
    use ratatui::{
        style::{Color, Modifier, Style},
        text::Line,
        widgets::{Block, Borders, Paragraph},
    };

    let mut lines = vec![
        Line::from(""),
        Line::styled(
            "Generation Complete",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
    ];

    if state.generated_count > 0 {
        lines.push(Line::from(format!(
            "  Generated: {} artifact(s)",
            state.generated_count
        )));
    }
    if state.skipped_count > 0 {
        lines.push(Line::from(format!(
            "  Skipped:   {} artifact(s)",
            state.skipped_count
        )));
    }
    if !state.failed.is_empty() {
        lines.push(Line::styled(
            format!("  Failed:    {} artifact(s)", state.failed.len()),
            Style::default().fg(Color::Red),
        ));
        for name in &state.failed {
            lines.push(Line::styled(
                format!("    - {}", name),
                Style::default().fg(Color::Red),
            ));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        "Press q to exit",
        Style::default().fg(Color::DarkGray),
    ));

    let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Done"));
    frame.render_widget(paragraph, area);
}

fn render_error_popup(frame: &mut Frame, error: &str) {
    use ratatui::{
        style::{Color, Style},
        widgets::{Block, Borders, Clear, Paragraph, Wrap},
    };

    let area = frame.area();

    // Center a popup
    let popup_area = centered_rect(60, 20, area);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let error_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title("Error");

    let error_text = Paragraph::new(error)
        .style(Style::default().fg(Color::Red))
        .wrap(Wrap { trim: true })
        .block(error_block);

    frame.render_widget(error_text, popup_area);
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    use ratatui::layout::{Constraint, Flex, Layout};

    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);

    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
