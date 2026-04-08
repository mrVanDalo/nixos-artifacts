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
   - Backend-specific settings
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

[backend_name.settings]            # Optional backend-specific config
key = "value"
another_key = 123
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
      serialization = "backend_name";
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
- `serialization`: Backend name (must exist in backend.toml)

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
│   │   └── test-skip-one/              # Test backend that skips one artifact
│   │       └── ...                     # Same structure as test/
│   └── scenarios/                      # Test scenarios (each is a complete flake)
│       ├── single-artifact-with-prompts/   # Simple scenario with prompts
│       ├── two-artifacts-no-prompts/       # Multiple artifacts, no prompts
│       ├── multiple-machines/              # Multi-machine NixOS setup
│       ├── home-manager/                   # Home-manager configuration
│       ├── artifact-name-formats/          # Various artifact naming patterns
│       ├── backend-include/                # Backend include directive test
│       ├── backend-circular-include/       # Circular include detection test
│       ├── no-config-section/              # Backend without [config] section
│       ├── error-missing-files/            # Error case: missing generated files
│       ├── error-missing-generator/        # Error case: missing generator
│       ├── error-unwanted-files/           # Error case: unwanted extra files
│       └── error-wrong-file-type/          # Error case: wrong file type
├── src/
│   ├── bin/
│   │   └── artifacts.rs     # CLI entry point
│   ├── app/                 # TUI application state (Elm Architecture)
│   │   ├── mod.rs           # Module exports
│   │   ├── model.rs         # State types (Model, Screen, PromptState)
│   │   ├── message.rs       # Event types (Msg, KeyEvent)
│   │   ├── effect.rs        # Side effect descriptors
│   │   └── update.rs        # Pure state transitions
│   ├── tui/                 # Terminal UI
│   │   ├── mod.rs           # Module exports
│   │   ├── views/           # Render functions
│   │   │   ├── list.rs      # Artifact list view
│   │   │   ├── prompt.rs    # Prompt input view
│   │   │   ├── progress.rs  # Generation progress view
│   │   │   └── generator_selection.rs  # Generator selection for shared artifacts
│   │   ├── events.rs        # EventSource trait + implementations
│   │   ├── runtime.rs       # Main loop, effect execution
│   │   ├── terminal.rs      # Terminal setup/teardown
│   │   ├── effect_handler.rs # Backend integration
│   │   └── model_builder.rs # Build Model from config
│   ├── backend/             # Backend operations
│   │   ├── generator.rs     # Generator script execution
│   │   ├── serialization.rs # Serialization operations
│   │   ├── helpers.rs       # Helper functions
│   │   └── temp_dir.rs      # Temporary directory management
│   ├── cli/                 # Command-line interface
│   │   ├── commands/        # Command implementations
│   │   ├── args.rs          # Argument parsing (clap)
│   │   └── mod.rs           # CLI orchestration
│   ├── config/              # Configuration management
│   │   ├── backend.rs       # Backend config parsing
│   │   ├── make.rs          # Make config parsing
│   │   └── nix.rs           # Nix evaluation helpers
│   ├── lib.rs               # Library root
│   └── macros.rs            # Utility macros
├── tests/
│   ├── tui/                 # TUI tests
│   │   ├── view_tests.rs    # View snapshot tests
│   │   └── snapshots/       # View snapshots
│   ├── cli/                 # CLI command tests
│   ├── backend/             # Backend tests
│   └── tests.rs             # Test entry point
├── Cargo.toml
└── CLAUDE.md
```

## Development Standards

### Coding Principles

1. **Fail fast** - Return errors early, don't continue with invalid state
2. **No abbreviations** - Use clear, descriptive names
3. **Function size** - Break long functions into smaller, sequential functions

### Rust Standards

- **Version**: Rust 1.87.0
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

**See also:** `src/backend/temp_dir.rs` for the project's temp directory
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
│   Update    │◀──── Msg (event)
│ (pure func) │────▶ Effect (side effect descriptor)
└─────────────┘
```

- **Model** (`app/model.rs`): Application state - Screen, artifacts, prompts
- **Msg** (`app/message.rs`): Events - keyboard input, async results
- **Effect** (`app/effect.rs`): Side effect descriptors (not executed in update)
- **Update** (`app/update.rs`): `(Model, Msg) -> (Model, Effect)` - pure!
- **View** (`tui/views/`): `(&Model) -> Frame` - pure rendering

### Key Types

```rust
// State
enum Screen { ArtifactList, SelectGenerator(..), Prompt(..), Generating(..), Done(..) }
enum InputMode { Line, Multiline, Hidden }
enum ArtifactStatus { Pending, NeedsGeneration, UpToDate, Generating, Done, Failed }

// List entries (for artifact list)
enum ListEntry { Single(ArtifactEntry), Shared(SharedEntry) }

// Events
enum Msg { Key(KeyEvent), Tick, GeneratorFinished{..}, SerializeFinished{..},
           SharedGeneratorFinished{..}, SharedSerializeFinished{..}, Quit }

// Side effects (descriptors, not actions)
enum Effect { None, Quit, CheckSerialization{..}, RunGenerator{..}, Serialize{..},
              ShowGeneratorSelection{..}, RunSharedGenerator{..}, SharedSerialize{..} }
```

### Runtime Loop (`tui/runtime.rs`)

```rust
loop {
    terminal.draw(|f| render(f, &model))?;  // View
    let msg = events.next_event()?;          // Get event
    let (model, effect) = update(model, msg); // Pure update
    execute_effect(effect)?;                  // Side effects
}
```

### Effect Handler (`tui/effect_handler.rs`)

Connects TUI to existing backend:

- `Effect::CheckSerialization` → `run_check_serialization()`
- `Effect::RunGenerator` → `run_generator_script()` + `verify_generated_files()`
- `Effect::Serialize` → `run_serialize()`
- `Effect::ShowGeneratorSelection` → Screen transition (no async work)
- `Effect::RunSharedGenerator` → `run_generator_script_with_path()`
- `Effect::SharedSerialize` → `run_shared_serialize()`

### Testing Patterns

**1. State transition tests** (fast, pure):

```rust
#[test]
fn test_navigate_down() {
    let model = make_test_model();
    let (new_model, effect) = update(model, Msg::Key(KeyEvent::char('j')));
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
        enter(),                    // Start generation
        ...type_string("secret"),   // Type prompt value
        enter(),                    // Submit
    ]);
    let final_model = simulate(&mut events, model);
    assert!(matches!(final_model.screen, Screen::Generating(_)));
}
```

### Adding a New Screen

1. Add variant to `Screen` enum in `app/model.rs`
2. Add state struct if needed (e.g., `NewScreenState`)
3. Handle in `update()` - add match arm for `(Screen::NewScreen, Msg::Key(_))`
4. Create view in `tui/views/new_screen.rs`
5. Add to dispatcher in `tui/views/mod.rs`
6. Write tests: state transitions + view snapshots

### Adding a New Effect

1. Add variant to `Effect` enum in `app/effect.rs`
2. Return it from `update()` when appropriate
3. Handle in `BackendEffectHandler::execute()`
4. Return result `Msg` to feed back into update loop

## Commands

### `generate` Command

**Arguments**:

- `backend.toml` - Path to backend configuration

**Working Directory**:

- Must be run in a directory containing `flake.nix`
- CLI automatically discovers and evaluates the flake
- Extracts `nixosConfigurations` and `homeConfigurations`

**Workflow** (for each artifact):

1. Create temporary `inputs` directory
2. Create file for every artifact file entry containing JSON with `path`,
   `owner`, `group`
3. Call `check` script:
   - Environment: `$artifact`, `$artifact_context`, `$targets`, `$inputs`, `$LOG_LEVEL`
   - Exit code 0: Skip to next artifact
   - Non-zero: Continue generation
4. Create temporary `prompts` directory
5. Create temporary `out` directory
6. Prompt user for input, save to `prompts` directory
7. Call `generator` script:
   - Execute in bubblewrap container
   - Environment: `$out`, `$prompts`, `$artifact`, `$artifact_context`, `$machine`/`$username` (context-dependent), `$LOG_LEVEL`
   - Verify success
   - Verify all demanded files generated
8. Call `serialize` script:
   - Environment: `$artifact`, `$artifact_context`, `$targets`, `$out`, `$LOG_LEVEL`
9. Remove temporary folders

**Implementation**: `src/cli/commands/generate.rs`

### `list` Command

List all artifacts defined in the configuration.

**Implementation**: `src/cli/commands/list.rs`

### `tui` Command

Launch interactive TUI for managing artifacts.

**Usage**:

```bash
artifacts                          # Show all artifacts (current directory as flake)
artifacts /path/to/flake           # Specify flake directory
artifacts --log-file /tmp/log.txt  # Enable debug logging
```

**Keybindings** (artifact list):

- `j`/`k` or arrows: Navigate
- `Enter`: Generate selected artifact
- `a`: Generate all artifacts
- `q`/`Esc`: Quit

**Keybindings** (prompt input):

- `Tab`: Cycle input mode (line/multiline/hidden) - only when empty
- `Enter`: Submit (line/hidden) or newline (multiline)
- `Ctrl+D`: Submit multiline input
- `Esc`: Cancel and return to list

**Implementation**: `src/cli/mod.rs` → `run_tui()`

### `help` Command

Print help message

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

- `tests/backend/` - Backend operation tests
- `tests/cli/` - CLI command tests
- Snapshots stored in respective `snapshots/` subdirectories

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

### Adding a New Command

1. Create command module in `src/cli/commands/`
2. Add argument parsing in `src/cli/args.rs`
3. Wire into `src/bin/artifacts.rs`
4. Add tests in `tests/cli/command_tests.rs` using insta-cmd pattern
5. Update this documentation

### Adding a Backend Operation

1. Define script paths in `backend.toml`
2. Implement caller in `src/backend/`
   - `generator.rs` - Generator script execution
   - `serialization.rs` - Serialization operations
   - `prompt.rs` - User prompt handling
3. Ensure bubblewrap container isolation
4. Pass required environment variables
5. Add error handling for script failures
6. Use helper functions from `src/backend/helpers.rs`

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

- **Language**: Rust 1.87.0
- **Entry point**: `src/bin/artifacts.rs`
- **TUI entry**: `src/cli/mod.rs` → `run_tui()`
- **Elm Architecture**: `src/app/` (model, message, effect, update)
- **TUI views**: `src/tui/views/` (list, prompt, progress)
- **Backend operations**: `src/backend/` directory
- **Backends**: `examples/backends/{test,test-skip-one,test-shared}/`
- **Test scenarios**: `examples/scenarios/{single-artifact-with-prompts,...}/`
- **Unit tests**: `cargo test --lib` (63 tests)
- **View snapshots**: `tests/tui/snapshots/` (13 snapshots)
- **Snapshot review**: `cargo insta review`
- **Container isolation**: bubblewrap for generator and serialize scripts

## Test Commands

```bash
cargo test --lib                    # Run all unit tests (63 tests)
cargo test app::                    # Test app module only
cargo test tui::                    # Test TUI module only
cargo test --test tests             # Run integration tests
cargo insta review                  # Review pending snapshots
cargo clippy                        # Run linter
```
