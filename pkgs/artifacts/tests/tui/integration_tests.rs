//! Integration tests for TUI using real example flakes.
//!
//! Each test follows the pattern:
//! 1. Load flake configuration
//! 2. Define event sequence
//! 3. Run with simulate() to test state transitions
//! 4. Snapshot the resulting model state
//!
//! Note: These tests verify UI state transitions only. Effect execution
//! (generator running, serialization) is tested in the e2e tests which
//! use the TestHarness to directly call backend operations.

use crate::test_helpers::*;
use crate::tui::model_state::ModelState;
use artifacts::app::Message;
use artifacts::config::backend::BackendConfiguration;
use artifacts::config::make::MakeConfiguration;
use artifacts::config::nix::build_make_from_flake;
use artifacts::tui::events::ScriptedEventSource;
use artifacts::tui::model_builder::build_model;
use artifacts::tui::simulate;
use insta::assert_debug_snapshot;
use serial_test::serial;
use std::collections::BTreeMap;
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

/// Generate a short random ID (8 hex characters).
fn short_uuid() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64,
    );
    format!("{:08x}", hasher.finish() as u32)
}

/// Create a unique temporary directory for test artifacts.
/// Retries up to 5 times with different UUIDs if creation fails.
fn create_test_output_dir(test_name: &str) -> PathBuf {
    let temp_dir = std::env::temp_dir();

    for _ in 0..5 {
        let unique_dir = temp_dir.join(format!("artifacts-test-{}-{}", test_name, short_uuid()));

        if std::fs::create_dir_all(&unique_dir).is_ok() {
            return unique_dir;
        }
    }

    panic!("Failed to create test output directory after 5 attempts");
}

/// Clean up the test output directory.
fn cleanup_test_output_dir(path: &PathBuf) {
    if path.exists() {
        let _ = std::fs::remove_dir_all(path);
    }
}

/// Collect serialized artifacts from the test output directory.
/// Returns a map of relative paths to file contents.
fn collect_serialized_artifacts(output_dir: &PathBuf) -> BTreeMap<String, String> {
    if !output_dir.exists() {
        return BTreeMap::new();
    }

    let mut artifacts = BTreeMap::new();
    collect_files_recursively(output_dir, output_dir, &mut artifacts);
    artifacts
}

/// Recursively collect all files from a directory tree.
fn collect_files_recursively(
    base_dir: &PathBuf,
    current_dir: &PathBuf,
    artifacts: &mut BTreeMap<String, String>,
) {
    let entries = match std::fs::read_dir(current_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            collect_files_recursively(base_dir, &path, artifacts);
            continue;
        }

        if !path.is_file() {
            continue;
        }

        let relative_path = path
            .strip_prefix(base_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        if let Ok(content) = std::fs::read_to_string(&path) {
            artifacts.insert(relative_path, content);
        }
    }
}

/// Extract test name from example path (e.g., "scenarios/single-artifact-with-prompts" -> "single-artifact-with-prompts")
fn extract_test_name(example: &str) -> &str {
    example.rsplit('/').next().unwrap_or(example)
}

fn run_tui(example: &str, events: Events) -> TestResult {
    let test_name = extract_test_name(example);
    let output_dir = create_test_output_dir(test_name);

    // Set the environment variable for serialize scripts to use
    // SAFETY: Tests run sequentially (not parallel) so there's no data race concern.
    // Each test has its own unique output directory based on test_name and a random UUID.
    unsafe {
        std::env::set_var("ARTIFACTS_TEST_OUTPUT_DIR", &output_dir);
    }

    let (_backend, make) = load_example(example);
    let model = build_model(&make);

    let before = ModelState::from_model(&model);

    // Use simulate() for pure state transition testing
    // Note: Effects are not executed - this tests UI state transitions only
    let mut event_source = ScriptedEventSource::new(events.messages);
    let final_model = simulate(&mut event_source, model);

    let after = ModelState::from_model(&final_model);

    // Collect serialized artifacts (will be empty since effects aren't executed)
    let serialized_artifacts = collect_serialized_artifacts(&output_dir);

    // Clean up the environment variable
    // SAFETY: Same as above - tests run sequentially.
    unsafe {
        std::env::remove_var("ARTIFACTS_TEST_OUTPUT_DIR");
    }

    // Clean up the test output directory
    cleanup_test_output_dir(&output_dir);

    TestResult {
        events: events.descriptions,
        before,
        after,
        serialized_artifacts,
    }
}

// =============================================================================
// Snapshot types (fields are read via Debug formatting in assert_debug_snapshot!)
// =============================================================================

// #[allow(dead_code)] needed because fields are read via Debug trait for snapshot testing
#[derive(Debug)]
#[allow(dead_code)]
struct TestResult {
    events: Vec<String>,
    before: ModelState,
    after: ModelState,
    serialized_artifacts: BTreeMap<String, String>,
}

// =============================================================================
// Chainable event builder
// =============================================================================

#[derive(Default)]
struct Events {
    messages: Vec<Message>,
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

    fn down(mut self) -> Self {
        self.messages.push(down());
        self.descriptions.push("down".to_string());
        self
    }

    fn up(mut self) -> Self {
        self.messages.push(up());
        self.descriptions.push("up".to_string());
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
fn multiple_machines_generate_all() {
    let events = Events::new().generate_all().quit();
    assert_debug_snapshot!(run_tui("scenarios/multiple-machines", events));
}

#[test]
#[serial]
fn multiple_machines_navigate_and_select() {
    let events = Events::new()
        .navigate_down(2)
        .select()
        .fill_prompts(&[])
        .quit();
    assert_debug_snapshot!(run_tui("scenarios/multiple-machines", events));
}

// =============================================================================
// Python scripts tests
// =============================================================================

#[test]
#[serial]
fn python_scripts_generate_all() {
    let events = Events::new().generate_all().quit();
    assert_debug_snapshot!(run_tui("scenarios/python-scripts", events));
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

#[test]
#[serial]
fn script_not_exists_shows_error() {
    // Script validation happens during check_serialization, which runs on startup
    let events = Events::new().quit();
    assert_debug_snapshot!(run_tui("scenarios/error-script-not-exists", events));
}

#[test]
#[serial]
fn script_not_executable_shows_error() {
    // Script validation happens during check_serialization, which runs on startup
    let events = Events::new().quit();
    assert_debug_snapshot!(run_tui("scenarios/error-script-not-executable", events));
}

#[test]
#[serial]
fn script_is_directory_shows_error() {
    // Script validation happens during check_serialization, which runs on startup
    let events = Events::new().quit();
    assert_debug_snapshot!(run_tui("scenarios/error-script-is-directory", events));
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

// =============================================================================
// shared-artifacts tests
// =============================================================================

#[test]
#[serial]
fn shared_artifacts_display() {
    // Test displays shared and single artifacts together
    let events = Events::new().quit();
    assert_debug_snapshot!(run_tui("scenarios/shared-artifacts", events));
}

#[test]
#[serial]
fn shared_artifacts_navigate() {
    // Navigate through mixed shared and single artifacts
    let events = Events::new().down().down().up().quit();
    assert_debug_snapshot!(run_tui("scenarios/shared-artifacts", events));
}

#[test]
#[serial]
fn shared_artifacts_generate_one() {
    // Select shared artifact, select generator, run generation
    // The shared artifact should transition through generator selection to generation
    let events = Events::new()
        .select() // Enter on shared artifact -> shows generator selection
        .select() // Enter on generator selection -> starts generation
        .quit();
    assert_debug_snapshot!(run_tui("scenarios/shared-artifacts", events));
}

// =============================================================================
// shared-artifacts error tests
// =============================================================================

#[test]
#[serial]
fn shared_artifacts_unwanted_files_shows_error() {
    // Test that shared artifacts which create wrong files produce an error
    // The generator creates "shared-unwanted" instead of "shared-key"
    let events = Events::new()
        .select() // Enter on shared artifact -> shows generator selection
        .select() // Enter on generator selection -> starts generation
        .quit();
    assert_debug_snapshot!(run_tui("scenarios/error-shared-unwanted-files", events));
}

// =============================================================================
// bubblewrap network isolation tests
// =============================================================================

/// Test that bubblewrap blocks network access in generators
/// The generator includes a "curl" call that will fail due to --unshare-all
#[test]
#[serial]
fn network_access_blocked_by_bubblewrap() {
    let events = Events::new().select().fill_prompts(&[]).quit();
    assert_debug_snapshot!(run_tui(
        "scenarios/error-bubblewrap-blocks-network-calls",
        events
    ));
}
