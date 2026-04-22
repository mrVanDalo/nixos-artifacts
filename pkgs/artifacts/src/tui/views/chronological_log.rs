use crate::app::model::{ChronologicalLogState, GenerationRun, LogEntry, LogFocus, LogLevel, Step};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn render_chronological_log(
    frame: &mut Frame,
    model: &crate::app::model::Model,
    state: &ChronologicalLogState,
    area: Rect,
) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(area);

    render_header(frame, state, chunks[0]);
    render_log_sections(frame, model, state, chunks[1]);
}

fn render_header(frame: &mut Frame, state: &ChronologicalLogState, area: Rect) {
    let header_text = vec![Line::from(vec![
        Span::styled(
            format!("Artifact: {} ", state.artifact_name),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("(Press 'q' to return, 'e' to expand all, 'c' to collapse all)"),
    ])];

    let header = Paragraph::new(header_text).block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, area);
}

fn render_log_sections(
    frame: &mut Frame,
    model: &crate::app::model::Model,
    state: &ChronologicalLogState,
    area: Rect,
) {
    let runs = match model.entries.get(state.artifact_index) {
        Some(entry) => entry.runs(),
        None => return,
    };

    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    render_scrollable_content(frame, state, runs, chunks[0]);
    render_legend(frame, chunks[1]);
}

fn render_scrollable_content(
    frame: &mut Frame,
    state: &ChronologicalLogState,
    runs: &[GenerationRun],
    area: Rect,
) {
    let mut lines: Vec<Line> = Vec::new();

    if runs.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no runs recorded yet)",
            Style::default().add_modifier(Modifier::DIM),
        )));
    }

    for (run_idx, run) in runs.iter().enumerate() {
        lines.push(run_header_line(state, run_idx, run));

        if !state.is_run_expanded(run_idx) {
            continue;
        }

        for step in Step::all_steps() {
            lines.push(step_header_line(state, run_idx, *step, run));
            if state.is_step_expanded(run_idx, *step) {
                for entry in run.step_logs.get(*step) {
                    lines.push(log_entry_line(entry));
                }
            }
        }
    }

    let paragraph = Paragraph::new(lines).scroll((state.scroll_offset as u16, 0));
    frame.render_widget(paragraph, area);
}

fn run_header_line(
    state: &ChronologicalLogState,
    run_idx: usize,
    run: &GenerationRun,
) -> Line<'static> {
    let is_expanded = state.is_run_expanded(run_idx);
    let is_focused = matches!(state.focus, Some(LogFocus::Run(r)) if r == run_idx);

    let expand_icon = if is_expanded { "▼" } else { "▶" };
    let focus_indicator = if is_focused { "→ " } else { "  " };
    let run_label = format!("Run {}", run_idx + 1);

    let mut spans = vec![
        Span::raw(focus_indicator),
        Span::raw(expand_icon.to_string()),
        Span::raw(" "),
        Span::styled(run_label, Style::default().add_modifier(Modifier::BOLD)),
    ];

    if !is_expanded {
        let summary = run_summary(run);
        spans.push(Span::raw(format!(" · {}", summary)));
    }

    Line::from(spans)
}

fn step_header_line(
    state: &ChronologicalLogState,
    run_idx: usize,
    step: Step,
    run: &GenerationRun,
) -> Line<'static> {
    let is_expanded = state.is_step_expanded(run_idx, step);
    let is_focused = matches!(state.focus, Some(LogFocus::Step(r, s)) if r == run_idx && s == step);

    let expand_icon = if is_expanded { "▼" } else { "▶" };
    let focus_indicator = if is_focused { "→ " } else { "  " };
    let logs = run.step_logs.get(step);
    let summary = calculate_summary(logs);

    Line::from(vec![
        Span::raw("    "),
        Span::raw(focus_indicator),
        Span::raw(expand_icon.to_string()),
        Span::raw(" "),
        Span::styled(step.label(), Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!(" ({})", summary)),
    ])
}

fn log_entry_line(entry: &LogEntry) -> Line<'static> {
    let prefix = match entry.level {
        LogLevel::Info => "[INFO] ",
        LogLevel::Output => "",
        LogLevel::Error => "[ERROR] ",
        LogLevel::Success => "[OK] ",
    };
    Line::from(vec![
        Span::raw("        "),
        Span::raw(prefix.to_string()),
        Span::raw(entry.message.clone()),
    ])
}

fn render_legend(frame: &mut Frame, area: Rect) {
    let legend_text = Line::from(vec![
        Span::styled("Space/Enter", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Toggle  "),
        Span::styled("+/-", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Expand/Collapse  "),
        Span::styled("j/k", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Navigate  "),
        Span::styled("PgUp/PgDn", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Scroll  "),
        Span::styled("Esc/q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": Back"),
    ]);

    let legend = Paragraph::new(legend_text).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(legend, area);
}

fn pluralize(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{} {}", count, singular)
    } else {
        format!("{} {}", count, plural)
    }
}

fn calculate_summary(logs: &[LogEntry]) -> String {
    let error_count = logs
        .iter()
        .filter(|l| matches!(l.level, LogLevel::Error))
        .count();
    let success_count = logs
        .iter()
        .filter(|l| matches!(l.level, LogLevel::Success))
        .count();

    let lines = pluralize(logs.len(), "line", "lines");
    if error_count > 0 {
        format!("{}, {}", lines, pluralize(error_count, "error", "errors"))
    } else if success_count > 0 {
        format!("{}, {} success", lines, success_count)
    } else {
        lines
    }
}

fn run_summary(run: &GenerationRun) -> String {
    let mut total_lines = 0usize;
    let mut total_errors = 0usize;
    for step in Step::all_steps() {
        let logs = run.step_logs.get(*step);
        total_lines += logs.len();
        total_errors += logs
            .iter()
            .filter(|l| matches!(l.level, LogLevel::Error))
            .count();
    }

    let lines = pluralize(total_lines, "line", "lines");
    if total_errors > 0 {
        format!("{}, {}", lines, pluralize(total_errors, "error", "errors"))
    } else {
        lines
    }
}
