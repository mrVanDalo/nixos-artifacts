mod generator_selection;
mod list;
mod progress;
mod prompt;

use crate::app::model::{Model, Screen, Warning};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

pub use generator_selection::render_generator_selection;
pub use list::render_artifact_list;
pub use progress::render_progress;
pub use prompt::render_prompt;

/// Top-level view dispatcher - renders the appropriate screen based on model state
pub fn render(frame: &mut Frame, model: &Model) {
    let area = frame.area();

    // If there are warnings, reserve space at the bottom for the banner
    let (content_area, warning_area) = if !model.warnings.is_empty() {
        let banner_height = std::cmp::min(model.warnings.len() as u16 + 2, 6);
        let chunks =
            Layout::vertical([Constraint::Min(0), Constraint::Length(banner_height)]).split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    match &model.screen {
        Screen::ArtifactList => render_artifact_list(frame, model, content_area),
        Screen::SelectGenerator(state) => render_generator_selection(frame, state, content_area),
        Screen::Prompt(state) => render_prompt(frame, state, content_area),
        Screen::Generating(state) => render_progress(frame, state, content_area),
        Screen::Done(state) => render_done(frame, state, content_area),
    }

    // Render warning banner if there are warnings
    if let Some(warning_area) = warning_area {
        render_warning_banner_to_area(frame, &model.warnings, warning_area);
    }

    // Render error popup if present (renders on top, after everything else)
    if let Some(ref error) = model.error {
        render_error_popup(frame, error);
    }
}

fn render_done(
    frame: &mut Frame,
    state: &crate::app::model::DoneState,
    area: ratatui::layout::Rect,
) {
    use ratatui::{
        style::{Color, Modifier, Style},
        text::Line,
        widgets::{Block, Borders, Paragraph},
    };

    let mut lines = vec![
        Line::from(""),
        Line::styled(
            "Generation Complete",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
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

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Done"));
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

fn render_warning_banner(frame: &mut Frame, warnings: &[Warning]) {
    // Legacy function - uses full frame area
    // Kept for backward compatibility, but main render() now uses render_warning_banner_to_area
    let area = frame.area();
    let banner_height = std::cmp::min(warnings.len() as u16 + 2, 6);
    let chunks =
        Layout::vertical([Constraint::Min(0), Constraint::Length(banner_height)]).split(area);
    render_warning_banner_to_area(frame, warnings, chunks[1]);
}

fn render_warning_banner_to_area(
    frame: &mut Frame,
    warnings: &[Warning],
    area: ratatui::layout::Rect,
) {
    use ratatui::{
        style::{Color, Style},
        text::Line,
        widgets::{Block, Borders, Paragraph},
    };

    let mut lines: Vec<Line> = warnings
        .iter()
        .take(4) // Show max 4 warnings
        .map(|w| {
            Line::styled(
                format!(" {} - {}", w.artifact_name, w.message),
                Style::default().fg(Color::Yellow),
            )
        })
        .collect();

    if warnings.len() > 4 {
        lines.push(Line::styled(
            format!(" ... and {} more warning(s)", warnings.len() - 4),
            Style::default().fg(Color::Yellow),
        ));
    }

    let warning_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title("Warnings");

    let warning_text = Paragraph::new(lines).block(warning_block);

    frame.render_widget(warning_text, area);
}

/// Helper function to create a centered rect
fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    area: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    use ratatui::layout::{Constraint, Flex, Layout};

    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);

    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
