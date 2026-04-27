use artifacts::app::message::{KeyEvent, Message};
use artifacts::app::model::{
    ArtifactEntry, ArtifactStatus, ChronologicalLogState, GenerationRun, ListEntry, LogEntry,
    LogFocus, LogLevel, Model, Screen, Step, StepLogs, TargetType,
};
use artifacts::app::update::update;
use artifacts::config::make::{ArtifactDef, FileDef};
use artifacts::tui::views::render_chronological_log;
use crossterm::event::KeyCode;
use insta::assert_snapshot;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn make_test_artifact(name: &str) -> ArtifactDef {
    ArtifactDef {
        name: name.to_string(),
        description: None,
        shared: false,
        files: BTreeMap::from([(
            "test".to_string(),
            FileDef {
                name: "test".to_string(),
                path: Some("/test/path".to_string()),
                owner: None,
                group: None,
            },
        )]),
        prompts: BTreeMap::new(),
        generator: "/nix/store/xxx-gen".to_string(),
        backend: "test-backend".to_string(),
    }
}

fn fixed_time(offset_secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_700_000_000 + offset_secs)
}

fn run_with_logs(offset_secs: u64, logs: StepLogs) -> GenerationRun {
    GenerationRun {
        started_at: fixed_time(offset_secs),
        step_logs: logs,
    }
}

fn logs_check_only() -> StepLogs {
    StepLogs {
        check: vec![
            LogEntry {
                level: LogLevel::Info,
                message: "Starting check".to_string(),
            },
            LogEntry {
                level: LogLevel::Success,
                message: "Already up to date".to_string(),
            },
        ],
        generate: vec![],
        serialize: vec![],
    }
}

fn logs_full_success() -> StepLogs {
    StepLogs {
        check: vec![LogEntry {
            level: LogLevel::Info,
            message: "Artifact stale".to_string(),
        }],
        generate: vec![
            LogEntry {
                level: LogLevel::Output,
                message: "Generating ssh key".to_string(),
            },
            LogEntry {
                level: LogLevel::Success,
                message: "Generated 1 file".to_string(),
            },
        ],
        serialize: vec![LogEntry {
            level: LogLevel::Success,
            message: "Serialized".to_string(),
        }],
    }
}

fn logs_with_errors() -> StepLogs {
    StepLogs {
        check: vec![LogEntry {
            level: LogLevel::Info,
            message: "Artifact missing".to_string(),
        }],
        generate: vec![
            LogEntry {
                level: LogLevel::Output,
                message: "Attempting generation".to_string(),
            },
            LogEntry {
                level: LogLevel::Error,
                message: "ssh-keygen failed".to_string(),
            },
        ],
        serialize: vec![],
    }
}

fn model_with_runs(runs: Vec<GenerationRun>) -> Model {
    let entry = ArtifactEntry {
        target_type: TargetType::NixOS {
            machine: "machine-one".to_string(),
        },
        artifact: make_test_artifact("ssh-key"),
        status: ArtifactStatus::UpToDate,
        runs,
    };

    let mut model = Model {
        screen: Screen::ArtifactList,
        entries: vec![ListEntry::Single(entry)],
        selected_index: 0,
        selected_log_step: Step::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
        generate_queue: Default::default(),
    };

    let num_runs = model.entries[0].runs().len();
    let state = ChronologicalLogState::new(0, "ssh-key".to_string(), num_runs);
    model.screen = Screen::ChronologicalLog(state);
    model
}

fn render(model: &Model) -> String {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let Screen::ChronologicalLog(state) = &model.screen else {
        panic!("expected ChronologicalLog screen");
    };
    terminal
        .draw(|f| render_chronological_log(f, model, state, f.area()))
        .unwrap();
    terminal.backend().to_string()
}

// ============================================================================
// Default state: latest run expanded, older runs collapsed
// ============================================================================

#[test]
fn test_empty_runs_shows_placeholder() {
    let model = model_with_runs(vec![]);
    assert_snapshot!(render(&model));
}

#[test]
fn test_single_run_latest_expanded_by_default() {
    let model = model_with_runs(vec![run_with_logs(0, logs_full_success())]);
    assert_snapshot!(render(&model));
}

#[test]
fn test_two_runs_only_latest_expanded() {
    let model = model_with_runs(vec![
        run_with_logs(0, logs_with_errors()),
        run_with_logs(60, logs_full_success()),
    ]);
    assert_snapshot!(render(&model));
}

#[test]
fn test_default_focus_is_latest_run_first_step() {
    let state = ChronologicalLogState::new(0, "ssh-key".to_string(), 2);
    assert_eq!(state.focus, Some(LogFocus::Step(1, Step::Check)));
    assert!(state.is_run_expanded(1));
    assert!(!state.is_run_expanded(0));
    assert!(state.is_step_expanded(1, Step::Check));
    assert!(state.is_step_expanded(1, Step::Generate));
    assert!(state.is_step_expanded(1, Step::Serialize));
}

#[test]
fn test_no_runs_has_no_focus() {
    let state = ChronologicalLogState::new(0, "ssh-key".to_string(), 0);
    assert_eq!(state.focus, None);
    assert!(state.expanded_runs.is_empty());
    assert!(state.expanded_steps.is_empty());
}

// ============================================================================
// Navigation (j/k)
// ============================================================================

fn send_key(model: Model, code: KeyCode) -> Model {
    let (m, _e) = update(model, Message::Key(KeyEvent::from_code(code)));
    m
}

fn focus_of(model: &Model) -> Option<LogFocus> {
    match &model.screen {
        Screen::ChronologicalLog(state) => state.focus,
        _ => None,
    }
}

#[test]
fn test_j_navigates_within_expanded_run() {
    let model = model_with_runs(vec![run_with_logs(0, logs_full_success())]);
    // Default focus is on (0, Check)
    assert_eq!(focus_of(&model), Some(LogFocus::Step(0, Step::Check)));

    let model = send_key(model, KeyCode::Char('j'));
    assert_eq!(focus_of(&model), Some(LogFocus::Step(0, Step::Generate)));

    let model = send_key(model, KeyCode::Char('j'));
    assert_eq!(focus_of(&model), Some(LogFocus::Step(0, Step::Serialize)));

    // Wraps to the run header at the top
    let model = send_key(model, KeyCode::Char('j'));
    assert_eq!(focus_of(&model), Some(LogFocus::Run(0)));
}

#[test]
fn test_j_crosses_run_boundary() {
    // Two runs, both expanded via expand-all
    let model = model_with_runs(vec![
        run_with_logs(0, logs_full_success()),
        run_with_logs(60, logs_full_success()),
    ]);
    let model = send_key(model, KeyCode::Char('+'));

    // Focus currently on (1, Check) — last step of last run
    let model = send_key(model, KeyCode::Char('k')); // (1, Check) -> run 1 header
    assert_eq!(focus_of(&model), Some(LogFocus::Run(1)));
    let model = send_key(model, KeyCode::Char('k')); // run 1 -> (0, Serialize)
    assert_eq!(focus_of(&model), Some(LogFocus::Step(0, Step::Serialize)));
    let model = send_key(model, KeyCode::Char('k')); // -> (0, Generate)
    assert_eq!(focus_of(&model), Some(LogFocus::Step(0, Step::Generate)));
    let model = send_key(model, KeyCode::Char('k')); // -> (0, Check)
    assert_eq!(focus_of(&model), Some(LogFocus::Step(0, Step::Check)));
    let model = send_key(model, KeyCode::Char('k')); // -> Run(0)
    assert_eq!(focus_of(&model), Some(LogFocus::Run(0)));
}

#[test]
fn test_j_skips_steps_of_collapsed_run() {
    // Two runs, default: only run 1 expanded
    let model = model_with_runs(vec![
        run_with_logs(0, logs_full_success()),
        run_with_logs(60, logs_full_success()),
    ]);
    // Focus is on (1, Check). Move up past run-1 header, into run-0
    let model = send_key(model, KeyCode::Char('k')); // -> Run(1)
    assert_eq!(focus_of(&model), Some(LogFocus::Run(1)));
    let model = send_key(model, KeyCode::Char('k')); // -> Run(0) (skipped collapsed steps)
    assert_eq!(focus_of(&model), Some(LogFocus::Run(0)));
}

// ============================================================================
// Toggle, expand-all, collapse-all
// ============================================================================

#[test]
fn test_toggle_focused_run() {
    let model = model_with_runs(vec![
        run_with_logs(0, logs_full_success()),
        run_with_logs(60, logs_full_success()),
    ]);
    // Move focus to Run(0)
    let model = send_key(model, KeyCode::Char('k'));
    let model = send_key(model, KeyCode::Char('k'));
    assert_eq!(focus_of(&model), Some(LogFocus::Run(0)));

    // Toggle: run 0 should now be expanded
    let model = send_key(model, KeyCode::Enter);
    if let Screen::ChronologicalLog(state) = &model.screen {
        assert!(state.is_run_expanded(0));
    }
}

#[test]
fn test_toggle_focused_step() {
    let model = model_with_runs(vec![run_with_logs(0, logs_full_success())]);
    // Default focus is (0, Check) and it's expanded
    let model = send_key(model, KeyCode::Char(' '));
    if let Screen::ChronologicalLog(state) = &model.screen {
        assert!(!state.is_step_expanded(0, Step::Check));
    }
}

#[test]
fn test_expand_all_key() {
    let model = model_with_runs(vec![
        run_with_logs(0, logs_full_success()),
        run_with_logs(60, logs_full_success()),
    ]);
    let model = send_key(model, KeyCode::Char('+'));
    if let Screen::ChronologicalLog(state) = &model.screen {
        assert!(state.is_run_expanded(0));
        assert!(state.is_run_expanded(1));
        for run in 0..2 {
            for step in Step::all_steps() {
                assert!(state.is_step_expanded(run, *step));
            }
        }
    }
}

#[test]
fn test_collapse_all_key() {
    let model = model_with_runs(vec![run_with_logs(0, logs_full_success())]);
    let model = send_key(model, KeyCode::Char('-'));
    if let Screen::ChronologicalLog(state) = &model.screen {
        assert!(state.expanded_runs.is_empty());
        assert!(state.expanded_steps.is_empty());
    }
}

// ============================================================================
// Snapshot: after expand-all with two runs
// ============================================================================

#[test]
fn test_two_runs_after_expand_all() {
    let model = model_with_runs(vec![
        run_with_logs(0, logs_with_errors()),
        run_with_logs(60, logs_full_success()),
    ]);
    let model = send_key(model, KeyCode::Char('+'));
    assert_snapshot!(render(&model));
}

#[test]
fn test_collapsed_run_shows_summary() {
    let model = model_with_runs(vec![
        run_with_logs(0, logs_with_errors()),
        run_with_logs(60, logs_check_only()),
    ]);
    // Default: run 1 expanded, run 0 collapsed → run 0 header shows summary
    assert_snapshot!(render(&model));
}
