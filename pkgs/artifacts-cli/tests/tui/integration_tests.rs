//! Integration tests for TUI using real example flakes.
//!
//! Each test follows the pattern:
//! 1. Load flake configuration
//! 2. Define event sequence
//! 3. Run with real effect handler and snapshot result

use artifacts_cli::app::model::Screen;
use artifacts_cli::app::Msg;
use artifacts_cli::config::backend::BackendConfiguration;
use artifacts_cli::config::make::MakeConfiguration;
use artifacts_cli::config::nix::build_make_from_flake;
use artifacts_cli::tui::events::test_helpers::*;
use artifacts_cli::tui::events::ScriptedEventSource;
use artifacts_cli::tui::model_builder::build_model;
use artifacts_cli::tui::{run, BackendEffectHandler};
use insta::assert_debug_snapshot;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use serial_test::serial;
use std::path::PathBuf;

// =============================================================================
// Test infrastructure
// =============================================================================

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_example(name: &str) -> (BackendConfiguration, MakeConfiguration) {
    let example_dir = project_root().join("examples").join(name);

    let backend = BackendConfiguration::read_backend_config(&example_dir.join("backend.toml"))
        .expect("Failed to read backend.toml");

    let make_path = build_make_from_flake(&example_dir).expect("Failed to build make from flake");
    let make =
        MakeConfiguration::read_make_config(&make_path).expect("Failed to read make config");

    (backend, make)
}

fn run_tui(example: &str, events: Vec<Msg>) -> TestResult {
    let (backend, make) = load_example(example);
    let model = build_model(&make);

    let before = ModelState::from_model(&model);

    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut event_source = ScriptedEventSource::new(events);
    let mut effect_handler = BackendEffectHandler::new(backend, make);

    let result = run(&mut terminal, &mut event_source, &mut effect_handler, model)
        .expect("TUI run failed");

    let after = ModelState::from_model(&result.final_model);

    TestResult { before, after }
}

// =============================================================================
// Snapshot types
// =============================================================================

#[derive(Debug)]
struct TestResult {
    before: ModelState,
    after: ModelState,
}

#[derive(Debug)]
struct ModelState {
    screen: &'static str,
    selected_index: usize,
    artifacts: Vec<ArtifactState>,
    error: Option<String>,
}

#[derive(Debug)]
struct ArtifactState {
    target: String,
    name: String,
    status: String,
}

impl ModelState {
    fn from_model(model: &artifacts_cli::app::model::Model) -> Self {
        Self {
            screen: match &model.screen {
                Screen::ArtifactList => "ArtifactList",
                Screen::Prompt(_) => "Prompt",
                Screen::Generating(_) => "Generating",
                Screen::Done(_) => "Done",
            },
            selected_index: model.selected_index,
            artifacts: model
                .artifacts
                .iter()
                .map(|a| ArtifactState {
                    target: a.target.clone(),
                    name: a.artifact.name.clone(),
                    status: format!("{:?}", a.status),
                })
                .collect(),
            error: model.error.clone(),
        }
    }
}

// =============================================================================
// Chainable event builder
// =============================================================================

#[derive(Default)]
struct Events(Vec<Msg>);

impl Events {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn navigate_down(mut self, n: usize) -> Self {
        self.0.extend((0..n).map(|_| down()));
        self
    }

    fn navigate_up(mut self, n: usize) -> Self {
        self.0.extend((0..n).map(|_| up()));
        self
    }

    fn select(mut self) -> Self {
        self.0.push(enter());
        self
    }

    fn fill_prompts(mut self, values: &[&str]) -> Self {
        for value in values {
            self.0.extend(type_string(value));
            self.0.push(enter());
        }
        self
    }

    fn type_and_cancel(mut self, partial: &str) -> Self {
        self.0.extend(type_string(partial));
        self.0.push(esc());
        self
    }

    fn generate_all(mut self) -> Self {
        self.0.push(char('a'));
        self
    }

    fn quit(mut self) -> Self {
        self.0.push(char('q'));
        self
    }

    fn build(self) -> Vec<Msg> {
        self.0
    }
}

// =============================================================================
// scenario_simple tests
// =============================================================================

#[test]
#[serial]
fn scenario_simple_generate_one() {
    let events = Events::new()
        .select()
        .fill_prompts(&["secret-one", "secret-two"])
        .quit()
        .build();
    assert_debug_snapshot!(run_tui("scenario_simple", events));
}

#[test]
#[serial]
fn scenario_simple_cancel_prompt() {
    let events = Events::new()
        .select()
        .type_and_cancel("partial-input")
        .quit()
        .build();
    assert_debug_snapshot!(run_tui("scenario_simple", events));
}

#[test]
#[serial]
fn scenario_simple_quit_immediately() {
    let events = Events::new().quit().build();
    assert_debug_snapshot!(run_tui("scenario_simple", events));
}

// =============================================================================
// 2_artifacts tests
// =============================================================================

#[test]
#[serial]
fn two_artifacts_generate_all() {
    let events = Events::new().generate_all().quit().build();
    assert_debug_snapshot!(run_tui("2_artifacts", events));
}

#[test]
#[serial]
fn two_artifacts_select_first() {
    let events = Events::new()
        .select()
        .fill_prompts(&[])
        .quit()
        .build();
    assert_debug_snapshot!(run_tui("2_artifacts", events));
}

#[test]
#[serial]
fn two_artifacts_select_second() {
    let events = Events::new()
        .navigate_down(1)
        .select()
        .fill_prompts(&[])
        .quit()
        .build();
    assert_debug_snapshot!(run_tui("2_artifacts", events));
}

// =============================================================================
// bigger_setup tests
// =============================================================================

#[test]
#[serial]
fn bigger_setup_generate_all() {
    let events = Events::new().generate_all().quit().build();
    assert_debug_snapshot!(run_tui("bigger_setup", events));
}

#[test]
#[serial]
fn bigger_setup_navigate_and_select() {
    let events = Events::new()
        .navigate_down(2)
        .select()
        .fill_prompts(&[])
        .quit()
        .build();
    assert_debug_snapshot!(run_tui("bigger_setup", events));
}

// =============================================================================
// Error scenario tests
// =============================================================================

#[test]
#[serial]
fn missing_files_shows_error() {
    let events = Events::new()
        .select()
        .fill_prompts(&["one", "two"])
        .quit()
        .build();
    assert_debug_snapshot!(run_tui("missing-files", events));
}

#[test]
#[serial]
fn wrong_file_type_shows_error() {
    let events = Events::new()
        .select()
        .fill_prompts(&["one", "two"])
        .quit()
        .build();
    assert_debug_snapshot!(run_tui("wrong-file-type", events));
}

#[test]
#[serial]
fn unwanted_files_shows_error() {
    let events = Events::new()
        .select()
        .fill_prompts(&["one", "two"])
        .quit()
        .build();
    assert_debug_snapshot!(run_tui("unwanted-files", events));
}

// =============================================================================
// Home manager tests
// =============================================================================

#[test]
#[serial]
fn simple_home_manager_generate_all() {
    let events = Events::new().generate_all().quit().build();
    assert_debug_snapshot!(run_tui("simple-home-manager", events));
}

// =============================================================================
// Backend configuration tests
// =============================================================================

#[test]
#[serial]
fn backend_include_generate_all() {
    let events = Events::new().generate_all().quit().build();
    assert_debug_snapshot!(run_tui("backend_include", events));
}

// =============================================================================
// Other scenarios
// =============================================================================

#[test]
#[serial]
fn artifact_names_generate_all() {
    let events = Events::new().generate_all().quit().build();
    assert_debug_snapshot!(run_tui("artifact_names", events));
}

#[test]
#[serial]
fn no_config_generate_one() {
    let events = Events::new()
        .select()
        .fill_prompts(&["one", "two"])
        .quit()
        .build();
    assert_debug_snapshot!(run_tui("no_config", events));
}
