//! Integration tests for TUI using real example flakes.
//!
//! Each test follows the pattern:
//! 1. Load flake configuration
//! 2. Define event sequence
//! 3. Run with real effect handler and snapshot result

use artifacts::app::Msg;
use artifacts::app::model::Screen;
use artifacts::config::backend::BackendConfiguration;
use artifacts::config::make::MakeConfiguration;
use artifacts::config::nix::build_make_from_flake;
use artifacts::tui::events::ScriptedEventSource;
use artifacts::tui::events::test_helpers::*;
use artifacts::tui::model_builder::build_model;
use artifacts::tui::{BackendEffectHandler, run};
use insta::assert_debug_snapshot;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
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
    let make = MakeConfiguration::read_make_config(&make_path).expect("Failed to read make config");

    (backend, make)
}

fn run_tui(example: &str, events: Events) -> TestResult {
    let (backend, make) = load_example(example);
    let model = build_model(&make);

    let before = ModelState::from_model(&model);

    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut event_source = ScriptedEventSource::new(events.messages);
    let mut effect_handler = BackendEffectHandler::new(backend, make);

    let result =
        run(&mut terminal, &mut event_source, &mut effect_handler, model).expect("TUI run failed");

    let after = ModelState::from_model(&result.final_model);

    TestResult {
        events: events.descriptions,
        before,
        after,
    }
}

// =============================================================================
// Snapshot types (fields are read via Debug formatting in assert_debug_snapshot!)
// =============================================================================

#[allow(dead_code)]
#[derive(Debug)]
struct TestResult {
    events: Vec<String>,
    before: ModelState,
    after: ModelState,
}

#[allow(dead_code)]
#[derive(Debug)]
struct ModelState {
    screen: &'static str,
    selected_index: usize,
    artifacts: Vec<ArtifactState>,
    error: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct ArtifactState {
    target: String,
    name: String,
    status: String,
}

impl ModelState {
    fn from_model(model: &artifacts::app::model::Model) -> Self {
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
struct Events {
    messages: Vec<Msg>,
    descriptions: Vec<String>,
}

impl Events {
    fn new() -> Self {
        Self::default()
    }

    fn navigate_down(mut self, n: usize) -> Self {
        self.messages.extend((0..n).map(|_| down()));
        self.descriptions.push(format!("navigate_down({})", n));
        self
    }

    fn select(mut self) -> Self {
        self.messages.push(enter());
        self.descriptions.push("select".to_string());
        self
    }

    fn fill_prompts(mut self, values: &[&str]) -> Self {
        for value in values {
            self.messages.extend(type_string(value));
            self.messages.push(enter());
        }
        if values.is_empty() {
            self.descriptions.push("fill_prompts([])".to_string());
        } else {
            self.descriptions
                .push(format!("fill_prompts({:?})", values));
        }
        self
    }

    fn type_and_cancel(mut self, partial: &str) -> Self {
        self.messages.extend(type_string(partial));
        self.messages.push(esc());
        self.descriptions
            .push(format!("type_and_cancel({:?})", partial));
        self
    }

    fn generate_all(mut self) -> Self {
        self.messages.push(char('a'));
        self.descriptions.push("generate_all".to_string());
        self
    }

    fn quit(mut self) -> Self {
        self.messages.push(char('q'));
        self.descriptions.push("quit".to_string());
        self
    }
}

// =============================================================================
// single-artifact-with-prompts tests (formerly scenario_simple)
// =============================================================================

#[test]
#[serial]
fn scenario_simple_generate_one() {
    let events = Events::new()
        .select()
        .fill_prompts(&["secret-one", "secret-two"])
        .quit();
    assert_debug_snapshot!(run_tui("scenarios/single-artifact-with-prompts", events));
}

#[test]
#[serial]
fn scenario_simple_cancel_prompt() {
    let events = Events::new()
        .select()
        .type_and_cancel("partial-input")
        .quit();
    assert_debug_snapshot!(run_tui("scenarios/single-artifact-with-prompts", events));
}

#[test]
#[serial]
fn scenario_simple_quit_immediately() {
    let events = Events::new().quit();
    assert_debug_snapshot!(run_tui("scenarios/single-artifact-with-prompts", events));
}

// =============================================================================
// two-artifacts-no-prompts tests (formerly 2_artifacts)
// =============================================================================

#[test]
#[serial]
fn two_artifacts_generate_all() {
    let events = Events::new().generate_all().quit();
    assert_debug_snapshot!(run_tui("scenarios/two-artifacts-no-prompts", events));
}

#[test]
#[serial]
fn two_artifacts_select_first() {
    let events = Events::new().select().fill_prompts(&[]).quit();
    assert_debug_snapshot!(run_tui("scenarios/two-artifacts-no-prompts", events));
}

#[test]
#[serial]
fn two_artifacts_select_second() {
    let events = Events::new()
        .navigate_down(1)
        .select()
        .fill_prompts(&[])
        .quit();
    assert_debug_snapshot!(run_tui("scenarios/two-artifacts-no-prompts", events));
}

// =============================================================================
// multiple-machines tests (formerly bigger_setup)
// =============================================================================

#[test]
#[serial]
fn bigger_setup_generate_all() {
    let events = Events::new().generate_all().quit();
    assert_debug_snapshot!(run_tui("scenarios/multiple-machines", events));
}

#[test]
#[serial]
fn bigger_setup_navigate_and_select() {
    let events = Events::new()
        .navigate_down(2)
        .select()
        .fill_prompts(&[])
        .quit();
    assert_debug_snapshot!(run_tui("scenarios/multiple-machines", events));
}

// =============================================================================
// Error scenario tests
// =============================================================================

#[test]
#[serial]
fn missing_files_shows_error() {
    let events = Events::new().select().fill_prompts(&["one", "two"]).quit();
    assert_debug_snapshot!(run_tui("scenarios/error-missing-files", events));
}

#[test]
#[serial]
fn wrong_file_type_shows_error() {
    let events = Events::new().select().fill_prompts(&["one", "two"]).quit();
    assert_debug_snapshot!(run_tui("scenarios/error-wrong-file-type", events));
}

#[test]
#[serial]
fn unwanted_files_shows_error() {
    let events = Events::new().select().fill_prompts(&["one", "two"]).quit();
    assert_debug_snapshot!(run_tui("scenarios/error-unwanted-files", events));
}

// =============================================================================
// Home manager tests
// =============================================================================

#[test]
#[serial]
fn simple_home_manager_generate_all() {
    let events = Events::new().generate_all().quit();
    assert_debug_snapshot!(run_tui("scenarios/home-manager", events));
}

// =============================================================================
// Backend configuration tests
// =============================================================================

#[test]
#[serial]
fn backend_include_generate_all() {
    let events = Events::new().generate_all().quit();
    assert_debug_snapshot!(run_tui("scenarios/backend-include", events));
}

// =============================================================================
// Other scenarios
// =============================================================================

#[test]
#[serial]
fn artifact_names_generate_all() {
    let events = Events::new().generate_all().quit();
    assert_debug_snapshot!(run_tui("scenarios/artifact-name-formats", events));
}

#[test]
#[serial]
fn no_config_generate_one() {
    let events = Events::new().select().fill_prompts(&["one", "two"]).quit();
    assert_debug_snapshot!(run_tui("scenarios/no-config-section", events));
}
