mod generator_selection;
mod list;
mod progress;
mod prompt;
mod regenerate_dialog;

mod chronological_log;
pub use chronological_log::render_chronological_log;

use crate::app::model::{Model, Screen, Warning};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

pub use generator_selection::render_generator_selection;
pub use list::render_artifact_list;
pub use progress::render_progress;
pub use prompt::render_prompt;
pub use regenerate_dialog::render_confirm_regenerate;

pub fn render(frame: &mut Frame, model: &Model) {
    let area = frame.area();

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
        Screen::ConfirmRegenerate(state) => render_confirm_regenerate(frame, state, content_area),
        Screen::Prompt(state) => render_prompt(frame, state, content_area),
        Screen::Generating(state) => render_progress(frame, state, content_area),
        Screen::Done(state) => render_done(frame, state, content_area),
        Screen::ChronologicalLog(state) => {
            render_chronological_log(frame, model, state, content_area)
        }
    }

    if let Some(warning_area) = warning_area {
        render_warning_banner_to_area(frame, &model.warnings, warning_area);
    }

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
        style::{Modifier, Style},
        text::Line,
        widgets::{Block, Borders, Paragraph},
    };

    let mut lines = vec![
        Line::from(""),
        Line::styled(
            "Generation Complete",
            Style::default().add_modifier(Modifier::BOLD),
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
            Style::default(),
        ));
        for name in &state.failed {
            lines.push(Line::styled(
                format!("    - {}", name),
                Style::default(),
            ));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::raw("Press q to exit"));

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Done"));
    frame.render_widget(paragraph, area);
}

fn render_error_popup(frame: &mut Frame, error: &str) {
    use ratatui::{
        widgets::{Block, Borders, Clear, Paragraph, Wrap},
    };

    let area = frame.area();

    let popup_area = centered_rect(60, 20, area);

    frame.render_widget(Clear, popup_area);

    let error_block = Block::default()
        .borders(Borders::ALL)
        .title("Error");

    let error_text = Paragraph::new(error)
        .wrap(Wrap { trim: true })
        .block(error_block);

    frame.render_widget(error_text, popup_area);
}

#[allow(dead_code)]
fn render_warning_banner(frame: &mut Frame, warnings: &[Warning]) {
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
        text::Line,
        widgets::{Block, Borders, Paragraph},
    };

    let mut lines: Vec<Line> = warnings
        .iter()
        .take(4)
        .map(|w| {
            Line::raw(format!(" {} - {}", w.artifact_name, w.message))
        })
        .collect();

    if warnings.len() > 4 {
        lines.push(Line::raw(format!(
            " ... and {} more warning(s)",
            warnings.len() - 4
        )));
    }

    let warning_block = Block::default()
        .borders(Borders::ALL)
        .title("Warnings");

    let warning_text = Paragraph::new(lines).block(warning_block);

    frame.render_widget(warning_text, area);
}

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