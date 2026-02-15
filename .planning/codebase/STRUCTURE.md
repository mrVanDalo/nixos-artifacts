# Codebase Structure

**Analysis Date:** 2025-02-13

## Directory Layout

```
/home/palo/dev/artifacts/nixos-artifacts/
├── flake.nix              # Flake entry, packages, nixosConfigurations
├── flake.lock             # Nix flake lockfile
├── modules/               # NixOS modules
│   ├── default.nix        # Module aggregation
│   ├── backend.nix        # Backend serialization options
│   ├── store.nix          # Artifacts store tree options
│   └── hm/                # Home-manager modules
│       ├── default.nix
│       ├── backend.nix
│       └── store.nix
├── pkgs/                  # Package definitions
│   └── artifacts/         # Rust CLI implementation
│       ├── Cargo.toml
│       ├── CLAUDE.md        # CLI-specific documentation
│       ├── default.nix      # Nix package definition
│       ├── examples/        # Test fixtures
│       │   ├── backends/    # Reusable backend definitions
│       │   │   ├── test/
│       │   │   ├── test-shared/
│       │   │   └── test-skip-one/
│       │   └── scenarios/   # Test scenarios (flakes)
│       ├── src/             # Rust source
│       │   ├── bin/
│       │   │   └── artifacts.rs    # CLI entry point
│       │   ├── app/         # Elm architecture (pure state)
│       │   │   ├── mod.rs
│       │   │   ├── model.rs        # State types
│       │   │   ├── message.rs      # Events
│       │   │   ├── effect.rs       # Side effect descriptors
│       │   │   └── update.rs       # Pure state transitions
│       │   ├── backend/     # Script execution
│       │   │   ├── mod.rs
│       │   │   ├── generator.rs    # Generator script runner
│       │   │   ├── serialization.rs
│       │   │   ├── prompt.rs
│       │   │   ├── helpers.rs
│       │   │   └── tempfile.rs
│       │   ├── cli/         # CLI layer
│       │   │   ├── mod.rs          # Orchestration
│       │   │   ├── args.rs         # Argument parsing
│       │   │   └── logging.rs
│       │   ├── config/      # Configuration parsing
│       │   │   ├── mod.rs
│       │   │   ├── backend.rs      # backend.toml parser
│       │   │   ├── make.rs         # make.json parser
│       │   │   └── nix.rs          # Nix evaluation
│       │   ├── tui/         # Terminal UI
│       │   │   ├── mod.rs
│       │   │   ├── runtime.rs      # Main loop
│       │   │   ├── effect_handler.rs
│       │   │   ├── events.rs
│       │   │   ├── model_builder.rs
│       │   │   ├── terminal.rs
│       │   │   └── views/          # Render functions
│       │   │       ├── mod.rs
│       │   │       ├── list.rs
│       │   │       ├── prompt.rs
│       │   │       ├── progress.rs
│       │   │       └── generator_selection.rs
│       │   ├── lib.rs
│       │   └── macros.rs
│       └── tests/           # Rust tests
│           ├── backend/
│           ├── cli/
│           └── tui/
├── examples/              # Example configurations
│   ├── default.nix
│   ├── simple-prompt.nix
│   ├── simple-generator.nix
│   ├── shared-generator.nix
│   ├── ideas/             # Design explorations
│   └── hm/                # Home-manager examples
├── nix/                   # Nix helper modules
│   ├── devshells.nix
│   ├── docs.nix
│   ├── formatter.nix
│   └── options.nix
├── docs/                  # Antora documentation
│   ├── antora.yml
│   └── modules/
│       └── ROOT/
│           ├── nav.adoc
│           ├── images/
│           ├── pages/
│           └── partials/
├── secrets/               # Generated secrets storage
│   ├── machines/
│   │   ├── machine-one/
│   │   └── machine-two/
│   └── shared/
└── backends/              # Backend implementations
    └── file/              # File-based backend (WIP)
```

## Directory Purposes

### `modules/`

NixOS module definitions. Defines the `artifacts.store` and `artifacts.backend`
option trees that users configure in their NixOS/home-manager configurations.

### `pkgs/artifacts/`

The Rust CLI implementation. This is a complete Rust project with its own
`CLAUDE.md`.

### `pkgs/artifacts/src/app/`

Elm Architecture implementation. Contains ONLY pure functions for state
management. No side effects, no I/O. Testable without mocking.

### `pkgs/artifacts/src/tui/`

Terminal UI implementation. Bridges the pure app layer with the terminal.
Handles events, effects, and rendering.

### `pkgs/artifacts/src/backend/`

Script execution layer. Runs generator and serialization scripts in bubblewrap
containers.

### `pkgs/artifacts/src/config/`

Configuration file parsing. Reads `backend.toml` and `make.json` into typed Rust
structs.

### `pkgs/artifacts/src/cli/`

Command-line interface. Argument parsing, path resolution, and orchestration.

### `pkgs/artifacts/examples/`

Test fixtures. `backends/` contains reusable backend definitions, `scenarios/`
contains complete test flakes.

### `examples/`

NixOS module examples showing how to use the artifacts framework.

### `nix/`

Nix helper modules imported by `flake.nix`.

### `docs/`

Antora-based documentation site.

### `secrets/`

Storage for generated secrets (not committed). Mirrors the shared/machines
distinction.

## Key File Locations

### Entry Points

- `pkgs/artifacts/src/bin/artifacts.rs` - Rust CLI binary entry
- `flake.nix` - Nix flake entry, defines packages and modules

### Configuration

- `modules/store.nix` - Artifact store options
- `modules/backend.nix` - Backend options
- `modules/hm/store.nix` - Home-manager store options
- `modules/hm/backend.nix` - Home-manager backend options

### Core Logic

- `pkgs/artifacts/src/cli/mod.rs` - CLI orchestration
- `pkgs/artifacts/src/tui/runtime.rs` - TUI event loop
- `pkgs/artifacts/src/app/update.rs` - State transitions
- `pkgs/artifacts/src/backend/generator.rs` - Generator execution
- `pkgs/artifacts/src/backend/serialization.rs` - Serialization execution

### Testing

- `pkgs/artifacts/tests/` - Test modules
- `pkgs/artifacts/tests/tui/snapshots/` - View snapshot tests
- `pkgs/artifacts/examples/scenarios/` - Integration test flakes

## Naming Conventions

### Files

- **Rust:** `snake_case.rs` (e.g., `effect_handler.rs`, `model_builder.rs`)
- **Nix:** `kebab-case.nix` (e.g., `simple-prompt.nix`, `devshells.nix`)
- **Tests:** `*_tests.rs` or `tests.rs` in module directories

### Directories

- **Rust modules:** `snake_case/` matching parent file
- **Nix examples:** `kebab-case/` for scenarios
- **Test scenarios:** descriptive kebab-case (e.g.,
  `single-artifact-with-prompts`, `error-missing-files`)

### Rust Types

- **Structs:** `PascalCase` (e.g., `ArtifactEntry`, `BackendConfiguration`)
- **Enums:** `PascalCase` (e.g., `Screen`, `Effect`, `InputMode`)
- **Functions:** `snake_case` (e.g., `run_generator_script`,
  `build_filtered_model`)
- **Constants:** `SCREAMING_SNAKE_CASE`

### Nix Options

- **Options:** `camelCase` for NixOS consistency (e.g., `checkSerialization`,
  `storeLocation`)

## Where to Add New Code

### New Artifact Option

- **Implementation:** `modules/store.nix` (add to `options` in artifact
  submodule)
- **For home-manager:** Also `modules/hm/store.nix`

### New Backend Option

- **Implementation:** `modules/backend.nix`
- **For home-manager:** Also `modules/hm/backend.nix`

### New CLI Command

1. Create in `pkgs/artifacts/src/cli/commands/`
2. Add to `pkgs/artifacts/src/cli/args.rs` (clap subcommand)
3. Wire in `pkgs/artifacts/src/cli/mod.rs`

### New Screen

1. Add variant to `Screen` enum in `pkgs/artifacts/src/app/model.rs`
2. Add state struct if needed
3. Handle in `pkgs/artifacts/src/app/update.rs`
4. Create view in `pkgs/artifacts/src/tui/views/<name>.rs`
5. Add to `pkgs/artifacts/src/tui/views/mod.rs` dispatcher

### New Effect

1. Add variant to `Effect` enum in `pkgs/artifacts/src/app/effect.rs`
2. Return from `update()` in `pkgs/artifacts/src/app/update.rs`
3. Handle in `pkgs/artifacts/src/tui/effect_handler.rs` `execute()`

### New Backend Script

- Add to `backends/<name>/` directory
- Reference in `backend.toml` with relative paths

### New Test Scenario

1. Create directory in `pkgs/artifacts/examples/scenarios/`
2. Add `flake.nix`, `backend.toml`, `flake.lock`
3. Include backend from `../backends/` or create custom

## Special Directories

### `pkgs/artifacts/examples/scenarios/`

- **Purpose:** Integration test fixtures
- **Generated:** No (hand-written)
- **Committed:** Yes
- **Naming:** Descriptive kebab-case

### `pkgs/artifacts/tests/tui/snapshots/`

- **Purpose:** View snapshot tests for TUI
- **Generated:** Yes (via `cargo insta`)
- **Committed:** Yes

### `build/site/`

- **Purpose:** Generated Antora documentation
- **Generated:** Yes (via `nix run .#build-docs`)
- **Committed:** No (in .gitignore)

### `secrets/`

- **Purpose:** Local storage for generated secrets
- **Generated:** Yes (by CLI)
- **Committed:** No (in .gitignore)

---

_Structure analysis: 2025-02-13_
