# Artifacts TUI - Development Guidelines

## Project Overview

artifacts-tui is a Terminal User Interface (TUI) application written in Rust for
generating, serializing, and deserializing secrets (aka artifacts) for NixOS
configurations. The tool provides a user-friendly interface for managing
artifacts through configurable backends.

## Architecture

### Core Components

1. **Backend Configuration (`backend.toml`)**
   - Defines serialization backends with custom scripts
   - Contains backend-specific settings
   - Supports multiple backends for different use cases

2. **Make Configuration (`make.json`)**
   - Extracted JSON from NixOS artifact store options
   - Defines artifacts, files, prompts, and their relationships
   - Contains metadata for secret generation and deployment

3. **TUI Interface**
   - Interactive terminal interface for secret management
   - Provides workflows for generation, serialization, and deserialization
   - User-friendly navigation and input handling

## Configuration Formats

### Backend Configuration Structure

```toml
[backend_name]
check_serialization = "/path/to/check/script"
deserialize = "/path/to/deserialize/script"
serialize = "/path/to/serialize/script"

[backend_name.settings]
key = "value"
another_key = 123
```

### Make Configuration Structure

The `make.json` file contains extracted NixOS options with the following
structure:

- **Artifacts**: Named collections of secrets
  - `name`: Artifact identifier
  - `shared`: Whether the artifact is shared across systems
  - `files`: File definitions for deployment
  - `prompts`: User input definitions
  - `generator`: Path to script for generating secrets
  - `serialization`: Backend references as string (check if backend is defined
    in `backend.toml`)

- **Files**: Deployment targets
  - `name`: File identifier
  - `path`: Target system path
  - `owner`: File permissions `owner`
  - `group`: File permissions `group`

- **Prompts**: User input definitions
  - `name`: Prompt identifier
  - `description`: User-facing description
  - `kind`: Input type (`hidden`, `line`, `multiline`)

## Development Standards

### Coding Guidelines

- fail fast
- don't use abbreviations
- when functions get log, try to split them up in a sequence of smaller
  functions

### Rust Guidelines

1. **Language Version**: Use Rust 1.87.0
2. **Linting**: Use `clippy` with default settings
3. **Error Handling**: Prefer `Result<T, E>` over panicking

#### Dependencies

- `clap` for command-line argument parsing
- `tui` for terminal user interface
- `serde` for serialization and deserialization
- `serde_json` for JSON serialization
- `serde_derive` for serialization and deserialization
- `insta_cmd` for snapshot testing
- `anyhow` for error handling
- `thiserror` for error handling
- `ratatui` for TUI components

### Testing Strategy

#### Unit Testing

- Test individual functions and modules
- Use `#[cfg(test)]` modules
- Aim for high test coverage on core logic

#### TUI Testing

- Test user interactions with the TUI
- Use `#[cfg(test)]` modules
- use the following pattern
- tests should run in serial order

```
#[cfg(test)]
mod tests {
    use insta_cmd::assert_cmd_snapshot;
    use insta_cmd::get_cargo_bin;
    use std::process::Command;

    fn cli() -> Command {
        Command::new(get_cargo_bin("artifacts-tui"))
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
    ...(other tests)
}
```

### Project Structure

```
examples/
└── scenarioA            # Scenario A
    ├── make.json        # make.json for scenario A
    ├── backend.toml     # backend.toml for scenario A
    ├── scriptA.sh       # referenced in make.json or backend.toml
    └── scriptB.sh       # referenced in make.json or backend.toml
src/
├── snapshots/           # insta-cmd Snapshots for testing
├── bin/                 # Binaries
│   ├── artifacts_cli.rs # Entry point for CLI
│   └── artifacts_tui.rs # Entry point for TUI
├── cli/                 # Command-line interface
│   ├── mod.rs
│   ├── commands/        # Individual commands
│   └── args.rs          # Argument parsing
├── tui/                 # Terminal UI components
│   ├── mod.rs
│   ├── app.rs           # Main application state
│   ├── components/      # UI components
│   └── events.rs        # Event handling
├── config/              # Configuration management
│   ├── mod.rs
│   ├── backend.rs       # Backend configuration
│   └── make.rs          # Make configuration
├── secrets/             # Secret management logic
│   ├── mod.rs
│   ├── generator.rs     # Secret generation
│   ├── serializer.rs    # Serialization
│   └── deserializer.rs  # Deserialization
├── tests/               # insta-cmd Tests
└── error.rs             # Error types
```

### Commands

artifacts-tui should have the following commands:

- `generate`: Generate artifacts
- `help`: Print help message

#### generate Command arguments

- backend.toml: Path to backend configuration file
- make.json: Path to make configuration file

#### generate Command workflow definition

artifacts-tui should have the following workflow for the `generate` command:

- for each artifact
  - create a temporary directory which will be referenced as `inputs`
  - create a file for every `file` entry of the artifact in a `inputs` temporary
    - containing json with `path`, `owner`, and `group`
  - call the `check_serialization` script
    - `inputs` directory injected as environment variable `$inputs`
    - `machine` and `artifact` injected as environment variables
  - if the `check_serialization` script succeeded with exit code 0, continue
    with the next artifact
  - if the `check_serialization` script failes with exit code different to 0,
    continue with the following steps
  - create a temporary directory which will be referenced as `prompts`
  - create a temporary directory which will be referenced as `out`
  - prompt the user for input for each prompt and save them in a file in a
    `prompts` temporary directory
  - call the `generator` script
    - execute the script in a bubblewrap container
      - `prompts` directory injected as environment variable `$prompts`
      - `out` directory injected as environment variable `$out`
    - verify if the generator script succeeded
    - verify if all demanded files were generated
  - call the `serialize` script defined in `backend.toml` referenced by the
    artifact
    - execute the script in a bubblewrap container`
    - `out` directory injected as environment variable `$out`
    - `machine` and `artifact` injected as environment variables
  - remove the temporary folders

## Linting

This project uses Clippy for linting. Treat warnings as errors.

- Run locally:
  - cargo lint
  - or ./test-lint.sh

Ensure you have the clippy component installed (via rustup):

- rustup component add clippy
