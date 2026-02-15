# Testing Patterns

**Analysis Date:** 2025-02-13

## Test Framework

**Runner:** Built-in `cargo test` (no custom runner) **Assertion Library:**
Standard Rust assertions + `anyhow::Result` for error handling **Snapshot
Testing:** `insta` and `insta-cmd` crates

**Key Dependencies:**

- `insta` (1.43.1) - Snapshot testing with filters
- `insta-cmd` (0.6) - Command-line snapshot testing
- `serial_test` (3) - Sequential test execution
- `tempfile` (3) - Temporary file/directory management
- `ratatui` (0.29) - TestBackend for TUI testing

**Run Commands:**

```bash
cargo test --lib                    # Run unit tests (63 tests)
cargo test app::                    # Test app module only
cargo test tui::                    # Test TUI module only
cargo test --test tests             # Run integration tests
cargo insta review                  # Review pending snapshots
cargo clippy                        # Run linter
```

## Test File Organization

**Location:**

- Unit tests: In-module with `#[cfg(test)]` blocks
- Integration tests: `tests/` directory parallel to `src/`
- TUI tests: `tests/tui/`
- Backend tests: `tests/backend/`

**Naming:**

- Test files: `*_tests.rs` or `mod.rs` in test directories
- Snapshots: Stored in `snapshots/` subdirectories with `.snap` extension

**Structure:**

```
pkgs/artifacts/
├── src/
│   └── lib.rs                      # Library root
├── tests/
│   ├── tests.rs                    # Test entry point
│   ├── tui/
│   │   ├── mod.rs
│   │   ├── integration_tests.rs    # Full TUI workflow tests
│   │   ├── view_tests.rs           # View snapshot tests
│   │   └── snapshots/              # View and integration snapshots
│   └── backend/
│       ├── mod.rs
│       └── snapshots/              # Backend helper snapshots
```

## Test Structure

**Suite Organization:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_descriptive_name() {
        // Arrange
        let input = setup_test_data();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected);
    }

    #[test]
    fn test_error_case() {
        let result = fallible_operation();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("expected error message"));
    }
}
```

**Serial Execution:**

```rust
use serial_test::serial;

#[test]
#[serial]  // Ensures tests don't run in parallel
fn test_that_modifies_global_state() {
    // Test code that uses env vars or shared resources
}
```

**Patterns:**

- Tests are organized by module/feature
- Descriptive test names: `test_what_is_being_tested`
- Setup with `TempDir` for filesystem tests
- Error message validation for failure cases

## Unit Testing

**In-Module Tests:**

```rust
// In src/app/model.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_mode_cycles() {
        let mode = InputMode::Line;
        assert_eq!(mode.next(), InputMode::Multiline);
        assert_eq!(mode.next().next(), InputMode::Hidden);
        assert_eq!(mode.next().next().next(), InputMode::Line);
    }

    #[test]
    fn test_target_type_context_str() {
        assert_eq!(TargetType::Nixos.context_str(), "nixos");
        assert_eq!(TargetType::HomeManager.context_str(), "homemanager");
    }
}
```

**State Machine Tests:**

```rust
#[test]
fn test_log_step_cycles() {
    let step = LogStep::Check;
    assert_eq!(step.next(), LogStep::Generate);
    assert_eq!(step.next().next(), LogStep::Serialize);
    assert_eq!(step.next().next().next(), LogStep::Check);
}
```

## Integration Testing

**Full TUI Simulation:** Located in `tests/tui/integration_tests.rs`

```rust
#[test]
#[serial]
fn scenario_simple_generate_one() {
    let events = Events::new()
        .select()
        .fill_prompts(&["secret-one", "secret-two"])
        .quit();
    assert_debug_snapshot!(run_tui("scenarios/single-artifact-with-prompts", events));
}
```

**Event Builder Pattern:**

```rust
#[derive(Default)]
struct Events {
    messages: Vec<Msg>,
    descriptions: Vec<String>,
}

impl Events {
    fn new() -> Self { Self::default() }
    
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
        self
    }
}
```

**Test Infrastructure:**

```rust
fn run_tui(example: &str, events: Events) -> TestResult {
    let test_name = extract_test_name(example);
    let output_dir = create_test_output_dir(test_name);
    
    // Set environment variable for test
    unsafe {
        std::env::set_var("ARTIFACTS_TEST_OUTPUT_DIR", &output_dir);
    }
    
    let (backend, make) = load_example(example);
    let model = build_model(&make);
    
    // Run TUI with scripted events
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut event_source = ScriptedEventSource::new(events.messages);
    let mut effect_handler = BackendEffectHandler::new(backend, make);
    
    let result = run(&mut terminal, &mut event_source, &mut effect_handler, model)
        .expect("TUI run failed");
    
    // Cleanup
    unsafe {
        std::env::remove_var("ARTIFACTS_TEST_OUTPUT_DIR");
    }
    cleanup_test_output_dir(&output_dir);
    
    TestResult { /* ... */ }
}
```

## Mocking

**No External Mocking Framework:**

- Uses trait abstractions for testability
- `EventSource` trait allows scripted event injection
- `EffectHandler` trait allows effect capture/verification

**ScriptedEventSource:**

```rust
#[derive(Debug, Default, Clone)]
pub struct ScriptedEventSource {
    events: VecDeque<Msg>,
}

impl EventSource for ScriptedEventSource {
    fn next_event(&mut self) -> Option<Msg> {
        self.events.pop_front()
    }
}
```

**Test Helpers:**

```rust
pub mod test_helpers {
    pub fn char(c: char) -> Msg {
        Msg::Key(KeyEvent::char(c))
    }
    
    pub fn enter() -> Msg {
        Msg::Key(KeyEvent::enter())
    }
    
    pub fn type_string(s: &str) -> Vec<Msg> {
        s.chars().map(char).collect()
    }
    
    pub fn submit_prompt(value: &str) -> Vec<Msg> {
        let mut events = type_string(value);
        events.push(enter());
        events
    }
}
```

## Fixtures and Factories

**Test Data Builders:**

```rust
fn make_test_artifact(name: &str, prompts: Vec<&str>) -> ArtifactDef {
    let mut prompt_map = BTreeMap::new();
    for p in prompts {
        prompt_map.insert(
            p.to_string(),
            PromptDef {
                name: p.to_string(),
                description: Some(format!("Enter the {} value", p)),
            },
        );
    }
    ArtifactDef {
        name: name.to_string(),
        shared: false,
        files: BTreeMap::from([("test".to_string(), FileDef { /* ... */ })]),
        prompts: prompt_map,
        generator: "/nix/store/xxx-gen".to_string(),
        serialization: "test-backend".to_string(),
    }
}

fn make_test_model() -> Model {
    let entry1 = ArtifactEntry { /* ... */ };
    let entry2 = ArtifactEntry { /* ... */ };
    
    Model {
        screen: Screen::ArtifactList,
        artifacts: vec![entry1.clone(), entry2.clone()],
        entries: vec![ListEntry::Single(entry1), ListEntry::Single(entry2)],
        // ...
    }
}
```

**Temporary Files:**

```rust
use tempfile::TempDir;

#[test]
fn test_validate_backend_script_not_exists() {
    let temp_dir = TempDir::new().unwrap();
    let result = validate_backend_script(
        "test-backend",
        "serialize",
        temp_dir.path(),
        "nonexistent.sh",
    );
    
    let err = result.unwrap_err().to_string();
    assert!(err.contains("does not exist"));
}
```

**Example Scenarios:**

- Located in `examples/scenarios/`
- Each is a complete flake with `flake.nix` and `backend.toml`
- Used for integration testing
- Examples: `single-artifact-with-prompts`, `multiple-machines`, `home-manager`

## Snapshot Testing

**View Snapshots:**

```rust
#[test]
fn test_artifact_list_initial() {
    let model = make_test_model();
    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    
    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();
    
    let result = ViewTestResult {
        state: ArtifactListState::from_model(&model),
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}
```

**Integration Snapshots:**

```rust
#[test]
#[serial]
fn scenario_simple_generate_one() {
    let events = Events::new()
        .select()
        .fill_prompts(&["secret-one", "secret-two"])
        .quit();
    assert_debug_snapshot!(run_tui("scenarios/single-artifact-with-prompts", events));
}
```

**Snapshot Review Workflow:**

```bash
cargo test                          # Generate new snapshots
cargo insta review                 # Interactive review/accept/reject
```

**Snapshot Location:**

- `tests/tui/snapshots/` for TUI tests
- `tests/backend/snapshots/` for backend tests
- Named with module path: `tests__tui__view_tests__test_name.snap`

## Coverage

**Requirements:** Not explicitly enforced **View Coverage:** Tests run with
`cargo test --lib`

**Coverage Areas:**

- 63+ unit tests for core logic
- 20+ integration tests for TUI workflows
- 13+ view snapshot tests for UI rendering

## Test Types

**Unit Tests:**

- Pure function testing
- State machine transitions
- Enum variant testing
- Helper function validation
- Located in `#[cfg(test)]` blocks within source files

**Integration Tests:**

- Full TUI workflow simulation
- Backend configuration parsing
- Error scenario testing (missing files, wrong types, etc.)
- Home-manager integration
- Multiple machine scenarios

**View Tests:**

- TUI rendering snapshot tests
- Test different states (initial, with selection, failed status)
- Use `TestBackend` from ratatui

**E2E Tests:**

- Command-line tests using `insta-cmd`
- Located in `tests/backend/helpers.rs` for CLI snapshot tests

## Common Patterns

**Async Testing:** Not used - all operations are synchronous with Effect
descriptors

**Error Testing:**

```rust
#[test]
fn test_validate_backend_script_not_exists() {
    let temp_dir = TempDir::new().unwrap();
    let result = validate_backend_script(
        "test-backend",
        "serialize",
        temp_dir.path(),
        "nonexistent.sh",
    );
    
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("backend 'test-backend' step 'serialize'"),
        "error should mention backend and step: {}",
        err
    );
    assert!(
        err.contains("does not exist"),
        "error should mention 'does not exist': {}",
        err
    );
}
```

**State Verification:**

```rust
#[test]
fn test_scripted_event_source() {
    let mut source = ScriptedEventSource::new(vec![char('a'), char('b'), enter()]);
    
    assert_eq!(source.len(), 3);
    assert!(!source.is_empty());
    
    assert!(matches!(source.next_event(), Some(Msg::Key(_))));
    assert!(source.next_event().is_none());
    assert!(source.is_empty());
}
```

**Environment Variable Handling:**

```rust
#[test]
#[serial]
fn test_with_env_var() {
    // SAFETY: Tests run sequentially (not parallel) so there's no data race concern
    unsafe {
        std::env::set_var("ARTIFACTS_TEST_OUTPUT_DIR", &output_dir);
    }
    
    // ... test code ...
    
    unsafe {
        std::env::remove_var("ARTIFACTS_TEST_OUTPUT_DIR");
    }
}
```

## Elm Architecture Testing

**Pure Update Testing:**

```rust
// State transitions are pure functions - easily testable
#[test]
fn test_navigate_down() {
    let model = make_test_model();
    let (new_model, effect) = update(model, Msg::Key(KeyEvent::char('j')));
    assert_eq!(new_model.selected_index, 1);
    assert!(effect.is_none());
}
```

**Effect Verification:**

- Effects are descriptors, not actions
- Verified through integration tests with real effect handlers

---

_Testing analysis: 2025-02-13_
