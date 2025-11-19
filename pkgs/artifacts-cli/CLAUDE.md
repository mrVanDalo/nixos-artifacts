# CLAUDE.md - AI Assistant Guide for Artifacts CLI

## Project Context

You are working on **artifacts-cli**, a Rust-based Command Line Interface for
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

```toml
[backend_name]
check_serialization = "/path/to/check/script"
deserialize = "/path/to/deserialize/script"
serialize = "/path/to/serialize/script"

[backend_name.settings]
key = "value"
another_key = 123
```

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
pkgs/artifacts-cli/
├── examples/                # Test scenarios (each is a complete flake)
│   ├── 2_artifacts/         # Multiple artifacts example
│   ├── artifact_names/      # Artifact naming example
│   ├── bigger_setup/        # Complex setup example
│   ├── missing-files/       # Error case: missing files
│   ├── missing_generator/   # Error case: missing generator
│   ├── no_config/           # Error case: no configuration
│   ├── scenario_simple/     # Simple scenario example
│   ├── simple-home-manager/ # Home-manager integration
│   ├── unwanted-files/      # Error case: unwanted files
│   └── wrong-file-type/     # Error case: wrong file type
│       ├── backend.toml     # Backend configuration
│       ├── flake.nix        # NixOS flake with artifacts
│       ├── flake.lock
│       ├── test_check.sh    # Check serialization script
│       └── test_serialize.sh # Serialization script
├── src/
│   ├── bin/
│   │   └── artifacts.rs     # CLI entry point
│   ├── backend/             # Backend operations
│   │   ├── generator.rs     # Generator script execution
│   │   ├── helpers.rs       # Helper functions
│   │   ├── prompt.rs        # User prompt handling
│   │   ├── serialization.rs # Serialization operations
│   │   └── temp_dir.rs      # Temporary directory management
│   ├── cli/                 # Command-line interface
│   │   ├── commands/
│   │   │   ├── generate.rs  # Generate command
│   │   │   ├── list.rs      # List command
│   │   │   └── mod.rs
│   │   ├── args.rs          # Argument parsing
│   │   ├── logging.rs       # Logging setup
│   │   └── mod.rs
│   ├── config/              # Configuration management
│   │   ├── backend.rs       # Backend config parsing
│   │   ├── make.rs          # Make config parsing
│   │   ├── make_expr.nix    # Nix expression for make config
│   │   ├── nix.rs           # Nix evaluation helpers
│   │   └── mod.rs
│   ├── error.rs             # Error types
│   ├── lib.rs               # Library root
│   └── macros.rs            # Utility macros
├── tests/                   # Integration tests
│   ├── backend/
│   │   ├── helpers.rs       # Backend test helpers
│   │   ├── snapshots/       # Backend test snapshots
│   │   └── mod.rs
│   ├── cli/
│   │   ├── command_tests.rs # CLI command tests
│   │   ├── snapshots/       # CLI test snapshots
│   │   └── mod.rs
│   └── tests.rs             # Test entry point
├── Cargo.toml               # Rust dependencies
├── default.nix              # Nix build file
└── CLAUDE.md                # CLAUDE.md for this project
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
- `insta_cmd` - Snapshot testing

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
3. Call `check_serialization` script:
   - Environment: `$inputs`, `$machine`, `$artifact`
   - Exit code 0: Skip to next artifact
   - Non-zero: Continue generation
4. Create temporary `prompts` directory
5. Create temporary `out` directory
6. Prompt user for input, save to `prompts` directory
7. Call `generator` script:
   - Execute in bubblewrap container
   - Environment: `$prompts`, `$out`
   - Verify success
   - Verify all demanded files generated
8. Call `serialize` script:
   - Execute in bubblewrap container
   - Environment: `$out`, `$machine`, `$artifact`
9. Remove temporary folders

**Implementation**: `src/cli/commands/generate.rs`

### `list` Command

List all artifacts defined in the configuration.

**Implementation**: `src/cli/commands/list.rs`

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

1. Create new directory in `examples/`
2. Add `flake.nix` with artifact configuration
3. Add `backend.toml` with test backend scripts
4. Create `test_check.sh` and `test_serialize.sh` scripts
5. Add `flake.lock` if needed
6. Document the scenario purpose (error case or feature demo)

## Quick Reference

- **Language**: Rust 1.87.0
- **Entry point**: `src/bin/artifacts.rs`
- **Test scenarios**: `examples/{2_artifacts,scenario_simple,bigger_setup,...}/`
- **Integration tests**: `tests/backend/`, `tests/cli/`
- **Snapshot tests**: `tests/backend/snapshots/`, `tests/cli/snapshots/`
- **Lint**: `cargo lint` or `./test-lint.sh`
- **Container isolation**: bubblewrap for generator and serialize scripts
- **Backend operations**: `src/backend/` directory
