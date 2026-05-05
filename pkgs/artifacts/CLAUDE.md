# CLAUDE.md - AI Assistant Guide for Artifacts CLI

## Project Context

You are working on **artifacts**, a Rust-based Command Line Interface for
generating, serializing, and deserializing secrets (artifacts) for NixOS
configurations. This tool manages artifacts through configurable backends with
interactive user prompts for secret generation.

## Core Architecture

### Two Main Components

1. **Backend Configuration (`backend.toml`)**
   - Defines serialization backends with custom scripts
   - Supports multiple backends (agenix, sops-nix, colmena, test)

2. **Flake Configuration (`flake.nix`)**
   - Contains `nixosConfigurations` and `homeConfigurations`
   - Each configuration defines `artifacts.store` options
   - CLI extracts artifacts, files, prompts, and relationships directly from the
     flake
   - Metadata for secret generation and deployment

## Configuration Reference

### backend.toml Structure

The `backend.toml` file uses a nested, target-centric structure with per-target
`enabled` capabilities:

```toml
[backend_name.nixos]
enabled = true                    # Optional, defaults to true if scripts set
check = "./check.sh"              # Optional, must pair with serialize
serialize = "./serialize.sh"     # Optional, must pair with check

[backend_name.home]
enabled = true
check = "./check.sh"
serialize = "./serialize.sh"

[backend_name.shared]
enabled = true
check = "./shared_check.sh"
serialize = "./shared_serialize.sh"
```

**Validation Rules:**

| `check` | `serialize` | Result                                         |
| ------- | ----------- | ---------------------------------------------- |
| absent  | absent      | Valid: `serializes = false` (passthrough mode) |
| present | present     | Valid: `serializes = true`                     |
| present | absent      | **ERROR**: "check requires serialize"          |
| absent  | present     | **ERROR**: "serialize requires check"          |

**`enabled` Inference Rules:**

| Condition                                        | Inferred `enabled` | Inferred `serializes`  |
| ------------------------------------------------ | ------------------ | ---------------------- |
| Section absent                                   | `false`            | N/A                    |
| Section present, no scripts, no `enabled`        | `false` (implicit) | `false`                |
| Section present, no scripts, `enabled = true`    | `true` (explicit)  | `false`                |
| Section present, both scripts, no `enabled`      | `true` (default)   | `true`                 |
| Section present, both scripts, `enabled = true`  | `true` (explicit)  | `true`                 |
| Section present, both scripts, `enabled = false` | `false` (explicit) | `true` (scripts exist) |

**Supported Targets:**

- `nixos`: NixOS machine configuration scripts
- `home`: Home-manager user configuration scripts
- `shared`: Shared artifact scripts (multi-machine artifacts)

**Shared Artifact Scripts:**

- `shared.serialize`: Called instead of `nixos.serialize` for shared artifacts
- `shared.check`: Called instead of `nixos.check` for shared artifacts
- Environment: `$artifact`, `$artifact_context`, `$targets` (JSON file), `$out`
  (serialize only), `$inputs` (check only), `$LOG_LEVEL`
- The `$targets` file contains a unified JSON structure with context, target
  names, types, and their backend configs

### Splitting backend.toml with Includes

Backend configuration can be split across multiple files using the `include`
directive. Paths are relative to the file containing the include.

```toml
# backend.toml
include = ["./backends/agenix.toml", "./backends/sops.toml"]

[test.nixos]
check = "./test_check.sh"
serialize = "./test_serialize.sh"

[test.home]
check = "./test_check.sh"
serialize = "./test_serialize.sh"
```

```toml
# backends/agenix.toml
[agenix.nixos]
check = "./agenix_check.sh"
serialize = "./agenix_serialize.sh"

[agenix.home]
check = "./agenix_check.sh"
serialize = "./agenix_serialize.sh"
```

**Include behavior:**

- Paths are resolved relative to the file containing the `include` directive
- Nested includes are supported (included files can include other files)
- Circular includes are detected and rejected
- Duplicate backend names across files produce an error
- Fully backwards compatible (files without `include` work unchanged)

### flake.nix Structure

The CLI extracts configuration from `nixosConfigurations` and
`homeConfigurations` in the flake:

```nix
{
  nixosConfigurations.my-machine = {
    # ... NixOS configuration
    artifacts.store.my-artifact = {
      files = { ... };
      prompts = { ... };
      generator = ./generator.sh;
      backend = "backend_name";
    };
  };

  homeConfigurations."user@host" = {
    # ... home-manager configuration
    artifacts.store.my-home-artifact = {
      # ... similar structure
    };
  };
}
```

**Artifacts** (defined in `artifacts.store.<name>`):

- `name`: Artifact identifier
- `shared`: Boolean - shared across systems or per-machine
- `files`: File definitions for deployment
- `prompts`: User input definitions
- `generator`: Path to the generator script (usually created with
  `pkgs.writers.writeBash`)
- `backend`: Backend name (must exist in backend.toml)

**Files**:

- `name`: File identifier
- `path`: Target system path
- `owner`: File permissions owner (only in `nixosConfigurations` context)
- `group`: File permissions group (only in `nixosConfigurations` context)

**Note**: In `homeConfigurations` context, `owner` and `group` are not available
since home-manager doesn't manage system-level permissions.

**Prompts**:

- `name`: Prompt identifier
- `description`: User-facing description

## Project Structure

```
pkgs/artifacts/
├── examples/
│   ├── backends/                       # Reusable backend definitions
│   │   ├── test/                       # Standard test backend (always passes)
│   │   │   ├── backend.toml            # Backend configuration with include
│   │   │   ├── check.sh                # Check serialization script
│   │   │   ├── serialize.sh            # Serialize script
│   │   │   └── deserialize.sh          # Deserialize script
│   │   ├── test-skip-one/              # Test backend that skips one artifact
│   │   │   └── ...                     # Same structure as test/
│   │   ├── test-shared/                # Test backend exercising shared scripts
│   │   └── test-config-verify/         # Test backend that asserts the unified $targets env interface
│   └── scenarios/                      # Test scenarios (each is a complete flake)
│       ├── single-artifact-with-prompts/   # Simple scenario with prompts
│       ├── two-artifacts-no-prompts/       # Multiple artifacts, no prompts
│       ├── multiple-machines/              # Multi-machine NixOS setup
│       ├── home-manager/                   # Mixed NixOS + home-manager config
│       ├── home-manager-only/              # Home-manager-only configuration
│       ├── shared-artifacts/               # Shared artifact across targets
│       ├── artifact-name-formats/          # Various artifact naming patterns
│       ├── backend-include/                # Backend include directive test
│       ├── backend-circular-include/       # Circular include detection test
│       ├── config-verify/                  # $targets env-var round-trip verification
│       ├── no-config-section/              # Minimal backend.toml (no per-target config)
│       ├── python-scripts/                 # Generator written in Python
│       ├── error-missing-files/            # Error: missing generated files
│       ├── error-missing-generator/        # Error: missing generator
│       ├── error-unwanted-files/           # Error: unwanted extra files
│       ├── error-shared-unwanted-files/    # Error: unwanted files for shared artifact
│       ├── error-wrong-file-type/          # Error: wrong file type
│       ├── error-script-not-exists/        # Error: backend script missing
│       ├── error-script-not-executable/    # Error: backend script not executable
│       ├── error-script-is-directory/      # Error: backend script path is a directory
│       └── error-bubblewrap-blocks-network-calls/  # Error: generator hits network
├── src/
│   ├── bin/
│   │   └── artifacts.rs                  # CLI entry point
│   ├── app/                              # Pure functional core (Elm Architecture)
│   │   ├── mod.rs                        # Module exports
│   │   ├── model/                        # State types
│   │   │   ├── core.rs                   # Model + Screen
│   │   │   ├── artifact.rs               # ListEntry, ArtifactStatus, GeneratingSubstate
│   │   │   ├── prompt.rs                 # PromptState, InputMode
│   │   │   ├── target.rs                 # TargetType
│   │   │   ├── log.rs                    # ChronologicalLogState, Step, Warning
│   │   │   └── screen_state.rs           # SelectGeneratorState, ConfirmRegenerateState, DoneState
│   │   ├── message.rs                    # Event types (Message, KeyEvent, ScriptOutput)
│   │   ├── effect.rs                     # Side effect descriptors (Effect, TargetSpec)
│   │   └── update/                       # Pure state transitions
│   │       ├── mod.rs                    # Top-level dispatch + pipeline pumping
│   │       ├── init.rs                   # Initial check fan-out
│   │       ├── artifact_list.rs          # Artifact list keybindings
│   │       ├── prompt.rs                 # Inline prompt handling
│   │       ├── confirm_regenerate.rs     # Regenerate dialog
│   │       ├── generator_selection.rs    # Generator selection dialog
│   │       ├── generating.rs             # Generation progress messages
│   │       ├── chronological_log.rs      # Log view navigation
│   │       └── tests.rs                  # Update-layer unit tests
│   ├── tui/                              # Terminal UI
│   │   ├── mod.rs                        # Module exports
│   │   ├── views/                        # Render functions
│   │   │   ├── list.rs                   # Artifact list view
│   │   │   ├── prompt.rs                 # Inline prompt view (right pane)
│   │   │   ├── progress.rs               # Generation progress (right pane)
│   │   │   ├── generator_selection.rs    # Generator selection dialog
│   │   │   ├── regenerate_dialog.rs      # Regenerate confirmation dialog
│   │   │   └── chronological_log.rs      # Chronological log screen
│   │   ├── events.rs                     # EventSource trait + implementations
│   │   ├── runtime.rs                    # Async main loop, effect dispatch
│   │   ├── terminal.rs                   # Terminal setup/teardown
│   │   ├── background.rs                 # BackgroundEffectHandler (FIFO task)
│   │   └── model_builder.rs              # Build Model from configuration
│   ├── backend/                          # Backend operations
│   │   ├── mod.rs                        # Module exports
│   │   ├── generator.rs                  # Generator script execution (bwrap)
│   │   ├── serialization.rs              # check / serialize execution
│   │   ├── helpers.rs                    # Helper functions
│   │   ├── output_capture.rs             # Streaming stdout/stderr capture
│   │   └── tempfile.rs                   # Temporary file/directory management
│   ├── cli/                              # Command-line interface
│   │   ├── args.rs                       # Argument parsing (clap)
│   │   └── mod.rs                        # CLI orchestration → run_tui()
│   ├── config/                           # Configuration management
│   │   ├── backend.rs                    # backend.toml parsing
│   │   ├── make.rs                       # Make JSON parsing
│   │   ├── nix.rs                        # Nix evaluation helpers
│   │   └── make_expr.nix                 # Nix expression that emits Make JSON
│   ├── lib.rs                            # Library root
│   ├── logging.rs                        # File-based logging + macros
│   └── macros.rs                         # Utility macros
├── tests/
│   ├── tests.rs                          # Test entry point (integration tests)
│   ├── test_helpers.rs                   # Shared helpers
│   ├── tui/                              # TUI tests (views, integration, model state)
│   │   ├── view_tests.rs                 # View snapshot tests
│   │   ├── integration_tests.rs          # End-to-end TUI flows
│   │   ├── chronological_log_tests.rs    # Log view tests
│   │   ├── regenerate_dialog_tests.rs    # Regenerate dialog tests
│   │   ├── model_state.rs                # Shared model fixtures
│   │   └── snapshots/                    # View snapshots
│   ├── cli/                              # CLI integration tests
│   ├── backend/                          # Backend tests
│   ├── config/                           # Config parsing tests
│   ├── async_tests/                      # Async runtime tests
│   ├── e2e/                              # End-to-end scenario tests
│   └── common/                           # Shared test utilities
├── Cargo.toml
└── CLAUDE.md
```

## Development Standards

### Coding Principles

1. **Fail fast** - Return errors early, don't continue with invalid state
2. **No abbreviations** - Use clear, descriptive names
3. **Function size** - Break long functions into smaller, sequential functions

### Rust Standards

- **Edition**: Rust edition 2024 (requires rustc ≥ 1.85)
- **Linting**: Clippy with default settings (treat warnings as errors)
- **Error Handling**: Use `anyhow::Result<T>` for application errors, avoid
  panicking
- **Testing**: Use `insta_cmd` for snapshot testing

### Key Dependencies

- `clap` - Command-line argument parsing
- `serde`, `serde_json`, `toml` - Serialization
- `anyhow` - Error handling
- `ratatui` - Terminal UI framework
- `crossterm` - Terminal manipulation
- `insta`, `insta_cmd` - Snapshot testing
- `tempfile` - Temporary file and directory management

### Temporary File Handling

**Always use the `tempfile` crate** for creating and managing temporary files
and directories. Never use manual `/tmp` paths or custom temp directory
creation.

**Why `tempfile`:**

- Automatically creates unique temporary paths to avoid collisions
- Secure permissions (restricted access by default)
- Automatic cleanup on drop (or persists if needed via `into_path()`)
- Cross-platform compatibility (works on Linux, macOS, Windows)
- Handles edge cases like tmpfs, noexec mounts, and disk space issues

**Common patterns:**

```rust
use tempfile::{NamedTempFile, TempDir};

// Single temporary file
let temp_file = NamedTempFile::new()?;
temp_file.write_all(b"content")?;
// File is automatically deleted when temp_file goes out of scope

// Temporary directory for multiple files
let temp_dir = TempDir::new()?;
let file_path = temp_dir.path().join("generated.txt");
std::fs::write(&file_path, "content")?;
// Directory and all contents deleted when temp_dir goes out of scope

// Keep tempfile after scope ends (for passing to external processes)
let temp_file = NamedTempFile::new()?;
temp_file.write_all(b"content")?;
let path = temp_file.keep()?; // File persists, returns PathBuf
```

**See also:** `src/backend/tempfile.rs` for the project's temp directory
utilities.

## TUI Architecture (Elm Architecture)

The TUI uses the **Elm Architecture** pattern for testability. All state
transitions are pure functions, and side effects are described as data.

### Core Concepts

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   Model     │────▶│    View      │────▶│   Frame     │
│  (state)    │     │ (pure func)  │     │ (rendered)  │
└─────────────┘     └──────────────┘     └─────────────┘
       ▲
       │
┌──────┴──────┐
│   Update    │◀──── Message (event)
│ (pure func) │────▶ Effect (side effect descriptor)
└─────────────┘
```

- **Model** (`app/model/`): Application state — Screen, entries, prompts, queues
- **Message** (`app/message.rs`): Events — keyboard input, async results, log
  nav
- **Effect** (`app/effect.rs`): Side effect descriptors (not executed in update)
- **Update** (`app/update/`): `(Model, Message) -> (Model, Effect)` — pure!
- **View** (`tui/views/`): `(&Model) -> Frame` — pure rendering

### Key Types

```rust
// State
enum Screen {
    ArtifactList,
    SelectGenerator(SelectGeneratorState),
    ConfirmRegenerate(ConfirmRegenerateState),
    Done(DoneState),
    ChronologicalLog(ChronologicalLogState),
}
enum InputMode { Line, Multiline, Hidden }
enum ArtifactStatus {
    Pending,
    NeedsGeneration,
    UpToDate,
    Generating(GeneratingSubstate),
    Failed { error: ArtifactError, .. },
    Cancelled { .. },
}

// List entries (for artifact list).
// Prompt collection and generation progress are NOT screens — prompts live
// inline via `Model.active_prompt`, progress renders in the right pane based
// on the entry's `ArtifactStatus::Generating(..)`.
enum ListEntry { Single(ArtifactEntry), Shared(SharedEntry) }

// Events
enum Message {
    Key(KeyEvent),
    Tick,
    CheckSerializationResult { artifact_index, status, result },
    GeneratorFinished      { artifact_index, result },
    GeneratorCancelled     { artifact_index },
    SerializeFinished      { artifact_index, result },
    GeneratorSelected      { artifact_index, generator_path },
    OutputLine             { artifact_index, stream, content },
    ToggleSection          { step },
    ScrollLogs             { delta },
    ExpandAllSections,
    CollapseAllSections,
    FocusNextSection,
    FocusPreviousSection,
    Quit,
}

// Unified target spec — single (one machine/user) or multi (shared artifact).
enum TargetSpec {
    Single(TargetType),
    Multi { nixos_targets: Vec<String>, home_targets: Vec<String> },
}

// Side effects (descriptors, not actions).
// All artifact-level effects carry a TargetSpec, so single and shared
// artifacts share the same Effect variants.
enum Effect {
    None,
    Batch(Vec<Self>),
    Quit,
    CheckSerialization { artifact_index, artifact_name, target_spec },
    RunGenerator       { artifact_index, artifact_name, target_spec, prompts },
    Serialize          { artifact_index, artifact_name, target_spec },
    CancelQueue,
}
```

### Runtime Loop (`tui/runtime.rs`)

The runtime is async. The TUI thread renders frames and pulls events; a single
`BackgroundEffectHandler` task (`tui/background.rs`) drains a FIFO of effects
and feeds results back as `Message`s.

```text
TUI thread                              Background task (FIFO)
──────────                              ──────────────────────
render(model)                           recv Effect
event = next_event()      ──Effect──▶   execute (run check / generator /
(model, effect) = update(model, event)    serialize, capture output)
dispatch_command(effect)  ◀──Message──  send Message (and OutputLine ticks
repeat                                    while the script runs)
```

`Effect::CancelQueue` is routed through a dedicated cancel channel so it can
drain the FIFO with priority; every other effect goes through the regular
command channel.

### Background Effect Handler (`tui/background.rs`)

`BackgroundEffectHandler` connects the TUI to the backend module. It owns the
`BackendConfiguration`, `MakeConfiguration`, and the per-artifact output
`TempDir`s (keyed by `artifact_index` so generator output cannot be clobbered
when the pipeline is mid-flight).

- `Effect::CheckSerialization` → `backend::serialization::check` →
  `Message::CheckSerializationResult`
- `Effect::RunGenerator` → `backend::generator::run` (+ generated-file
  verification) → `Message::GeneratorFinished` (or `Message::GeneratorCancelled`
  on user cancel)
- `Effect::Serialize` → `backend::serialization::serialize` →
  `Message::SerializeFinished`
- `Effect::CancelQueue` → drain pending FIFO entries; the in-flight generator's
  bwrap process group is signalled via the handler's `KillSlot` (serialize is
  allowed to finish so the backend never sees a half-written artifact).

Single vs. shared artifacts share the same Effect variants; the handler
dispatches on `TargetSpec::Single` vs. `TargetSpec::Multi`.

### Testing Patterns

**1. State transition tests** (fast, pure):

```rust
#[test]
fn test_navigate_down() {
    let model = make_test_model();
    let (new_model, effect) = update(model, Message::Key(KeyEvent::char('j')));
    assert_eq!(new_model.selected_index, 1);
    assert!(effect.is_none());
}
```

**2. View snapshot tests** (using TestBackend):

```rust
#[test]
fn test_prompt_view() {
    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| render_prompt(f, &state, f.area())).unwrap();
    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}
```

**3. Simulation tests** (scripted event sequences):

```rust
#[test]
fn test_complete_flow() {
    let mut events = ScriptedEventSource::new(vec![
        enter(),                    // Start generation (opens inline prompt)
        type_string("secret"),      // Type prompt value
        enter(),                    // Submit prompt; generator effect emitted
    ]);
    let final_model = simulate(&mut events, model);
    // simulate() does not execute effects — assert on Model state instead.
    assert!(matches!(final_model.screen, Screen::ArtifactList));
    assert!(final_model.active_prompt.is_none());
}
```

### Adding a New Screen

1. Add variant to `Screen` enum in `app/model/core.rs`
2. Add state struct if needed (e.g., `NewScreenState`)
3. Handle in `update()` — add match arm for
   `(Screen::NewScreen, Message::Key(_))`
4. Create view in `tui/views/new_screen.rs`
5. Add to dispatcher in `tui/views/mod.rs`
6. Write tests: state transitions + view snapshots

NOTE: prompt collection and generation progress are **not** screens — prompts
live inline via `Model.active_prompt` and generation progress renders in the
right pane based on `ArtifactStatus::Generating(GeneratingSubstate)`. Prefer
that pattern over adding new screens for transient UI states.

### Adding a New Effect

1. Add variant to `Effect` enum in `app/effect.rs`
2. Return it from `update()` when appropriate
3. Handle in `BackgroundEffectHandler::execute()` (`tui/background.rs`)
4. Return a result `Message` to feed back into the update loop

## CLI

The binary takes no subcommands. Invoking `artifacts` always launches the
interactive TUI; the only positional argument is an optional flake path, plus
`--log-file` / `--log-level` for diagnostics.

```bash
artifacts                                      # Use current directory as flake
artifacts /path/to/flake                       # Point at another flake directory
artifacts --log-file /tmp/log.txt              # Enable file logging
artifacts --log-file /tmp/log.txt --log-level debug
```

**Implementation**: `src/cli/mod.rs` → `run()` → `run_tui()`. CLI flags are
defined in `src/cli/args.rs`.

### Lifecycle (per artifact, executed by the runtime + background task)

1. Resolve `flake.nix` and `backend.toml`, build the Make JSON via
   `config::nix::build_make_from_flake`.
2. `init` dispatches an initial `Effect::CheckSerialization` for every entry.
   The background task runs the backend `check` script with `$artifact`,
   `$artifact_context`, `$targets`, `$inputs`, `$LOG_LEVEL` — exit code 0 marks
   the entry `UpToDate`, non-zero marks `NeedsGeneration`.
3. The user (or the `a` flow) triggers `Effect::RunGenerator`. The handler
   creates a temp `out` dir and a temp `prompts` dir, writes prompt values to
   files, and runs the generator inside a bubblewrap container. The exported
   environment depends on the artifact kind:
   - **Per-target generators** (NixOS or Home Manager): `$out`, `$prompts`,
     `$artifact`, `$artifact_context` (`"nixos"` or `"homemanager"`), `$machine`
     _or_ `$username`, `$LOG_LEVEL`.
   - **Shared generators**: `$out`, `$prompts`, `$artifact_context = "shared"`,
     `$LOG_LEVEL` only. Shared generators do **not** receive `$artifact`,
     `$machine`, or `$username` — see `build_shared_env_exports` in
     `src/backend/generator.rs`.
4. Generated files are verified against the artifact's `files` schema.
5. `Effect::Serialize` runs the backend `serialize` script with `$artifact`,
   `$artifact_context`, `$targets`, `$out`, `$LOG_LEVEL`.
6. Temp directories are dropped on success or failure.

Generators run sequentially in a single FIFO background task — parallelism would
change the user-visible gen→ser→gen→ser order and is deliberately not
introduced.

### Keybindings

**Artifact list** (`Screen::ArtifactList`)

- `j`/`k` or arrows: Navigate the list
- `Tab`: Cycle the selected log step in the right pane
- `Enter`: Generate (or re-generate via the confirm dialog) the selected
  artifact
- `a`: Generate every artifact that needs generation
- `l`: Open the chronological log view for the selected artifact
- `Esc`: First press arms the universal cancel chord
- `Esc Esc` (within 500ms): Cancel queued generators / abort the in-flight one
- `q`: Quit

**Inline prompt** (right pane when `Model.active_prompt` is `Some`)

- `Tab` (only when buffer is empty): Cycle input mode (line / multiline /
  hidden)
- `Enter`: Submit (line/hidden) or insert a newline (multiline)
- `Ctrl+D`: Submit multiline input
- `Esc`: Skip the current artifact (during the `a` flow) or cancel the prompt
- `Esc Esc` (within 500ms): Cancel the entire `a`-flow queue

**Generator selection** (`Screen::SelectGenerator`)

- `j`/`k` or arrows: Move between candidate generators
- `Enter`: Confirm the highlighted generator
- `Esc` / `q`: Cancel and return to the artifact list

**Confirm regenerate** (`Screen::ConfirmRegenerate`)

- `Left`/`h` ↔ `Right`/`l`, or `Tab`: Toggle between Leave / Regenerate
- `Enter` / `Space`: Apply the highlighted choice
- `Esc`: Cancel (equivalent to Leave)

**Chronological log** (`Screen::ChronologicalLog`)

- `j` / `Down` / `Tab`: Focus next section
- `k` / `Up`: Focus previous section
- `PageDown` / `PageUp`: Scroll the log content
- `Enter` / `Space`: Toggle the focused section
- `+` / `=` / `e`: Expand all sections
- `-` / `c`: Collapse all sections
- `Esc` / `q`: Back to the artifact list

## Testing Strategy

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    // Test individual functions and modules
    // Aim for high test coverage on core logic
}
```

### Integration Testing (insta-cmd pattern)

Located in `tests/` directory:

```rust
#[cfg(test)]
mod tests {
    use insta_cmd::assert_cmd_snapshot;
    use insta_cmd::get_cargo_bin;
    use std::process::Command;

    fn cli() -> Command {
        Command::new(get_cargo_bin("artifacts"))
    }

    #[test]
    #[serial]
    fn test_main_no_arguments() {
        assert_cmd_snapshot!(cli());
    }

    #[test]
    #[serial]
    fn test_main_help() {
        assert_cmd_snapshot!(cli().arg("--help"));
    }
}
```

**Important**: Tests should run in serial order

**Snapshot Review Workflow**: When running tests that generate new snapshots,
run `cargo test` without accepting snapshots automatically. Then inform the user
to run `cargo insta review` in a separate terminal to review and accept/reject
the snapshots manually. NEVER run `cargo insta accept` or
`cargo insta test --accept`.

**Test Organization**:

- `tests/tui/` — view snapshots, integration flows, model fixtures
- `tests/cli/` — top-level CLI integration tests (insta-cmd)
- `tests/backend/` — backend operation tests
- `tests/config/` — config parsing tests
- `tests/async_tests/` — async runtime tests
- `tests/e2e/` — end-to-end scenario tests against `examples/scenarios/`
- Snapshots are stored in each subdirectory's `snapshots/` folder

## Linting

Run linting before committing:

```bash
cargo lint
```

Run clippy before commiting:

```bash
cargo clippy
```

## Common Tasks

### Adding a Backend Operation

1. Define script paths in `backend.toml`
2. Implement the caller in `src/backend/`
   - `generator.rs` — Generator script execution (bubblewrap)
   - `serialization.rs` — `check` / `serialize` execution
   - `output_capture.rs` — Streaming stdout/stderr capture used by both
3. Keep bubblewrap container isolation for any process running user code
4. Pass required environment variables (see Lifecycle section above)
5. Add error handling for script failures (return `ArtifactError`)
6. Use helper functions from `src/backend/helpers.rs` and the `tempfile`-based
   utilities in `src/backend/tempfile.rs`

### Working with Configuration

- Parse backend.toml: `src/config/backend.rs`
- Extract from flake.nix: `src/config/make.rs`
- Nix evaluation: `src/config/nix.rs` and `src/config/make_expr.nix`
- CLI must be run in directory containing `flake.nix`
- CLI evaluates flake to extract `nixosConfigurations` and `homeConfigurations`
- Validate serialization backend exists in both backend.toml and flake
- Check artifact references are valid
- Handle differences between `nixosConfigurations` (with owner/group) and
  `homeConfigurations` (without owner/group)

### Adding Test Scenarios

1. Create new directory in `examples/scenarios/`
2. Add `flake.nix` with artifact configuration
3. Add `backend.toml` that includes a backend from `../backends/`:
   ```toml
   include = ["../backends/test/backend.toml"]
   ```
4. Create `test_check.sh` and `test_serialize.sh` scripts for testing
5. Add `flake.lock` if needed
6. Use descriptive kebab-case naming:
   - Feature demos: `single-artifact-with-prompts`, `multiple-machines`
   - Error cases: `error-missing-files`, `error-wrong-file-type`

## Quick Reference

- **Language**: Rust (edition 2024, rustc ≥ 1.85)
- **Entry point**: `src/bin/artifacts.rs`
- **TUI entry**: `src/cli/mod.rs` → `run()` → `run_tui()`
- **Elm Architecture**: `src/app/` (`model/`, `message.rs`, `effect.rs`,
  `update/`)
- **TUI views**: `src/tui/views/` (list, prompt, progress, generator_selection,
  regenerate_dialog, chronological_log)
- **Background task**: `src/tui/background.rs` (`BackgroundEffectHandler`)
- **Backend operations**: `src/backend/` (generator, serialization, helpers,
  output_capture, tempfile)
- **Example backends**:
  `examples/backends/{test,test-skip-one,test-shared,test-config-verify}/`
- **Test scenarios**: `examples/scenarios/` (one directory per scenario; see the
  Project Structure tree above for the full list)
- **Unit tests**: `cargo test --lib`
- **View snapshots**: `tests/tui/snapshots/`
- **Snapshot review**: `cargo insta review`
- **Container isolation**: bubblewrap for generator scripts

## Test Commands

```bash
cargo test --lib                    # Run all unit tests
cargo test app::                    # Test app module only
cargo test tui::                    # Test TUI module only
cargo test --test tests             # Run integration tests
cargo insta review                  # Review pending snapshots
cargo clippy                        # Run linter
```
