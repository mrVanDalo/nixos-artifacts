# Architecture

**Analysis Date:** 2025-02-13

## Pattern Overview

**Overall:** Elm Architecture (Model-Update-View-Effect) for the TUI, with a
declarative NixOS module system for configuration.

**Key Characteristics:**

- Pure functional state management with side effects described as data
- Separation between NixOS module declarations and Rust CLI execution
- Backend plugin system via shell script contracts
- Shared vs per-target artifact distinction
- Bubblewrap container isolation for generator and serialization scripts

## Layers

### NixOS Module Layer

- **Purpose:** Declarative artifact definitions via NixOS options
- **Location:** `modules/`, `modules/hm/`
- **Contains:** Option schemas for `artifacts.store`, `artifacts.backend`
- **Depends on:** NixOS module system, home-manager
- **Used by:** Flake to generate Make JSON, Rust CLI to consume

### Configuration Parsing Layer

- **Purpose:** Parse backend.toml and make.json into Rust structs
- **Location:** `pkgs/artifacts/src/config/`
- **Contains:**
  - `backend.rs` - Backend script definitions with include support
  - `make.rs` - Artifact definitions from NixOS configurations
  - `nix.rs` - Nix flake evaluation helpers
- **Depends on:** serde, toml, serde_json
- **Used by:** CLI module, TUI model builder

### Elm Architecture Layer (App)

- **Purpose:** Pure state transitions and effect descriptors
- **Location:** `pkgs/artifacts/src/app/`
- **Contains:**
  - `model.rs` - State types (Model, Screen, PromptState, ArtifactStatus)
  - `message.rs` - Event types (Msg, KeyEvent)
  - `effect.rs` - Side effect descriptors (Effect enum)
  - `update.rs` - Pure update function `(Model, Msg) -> (Model, Effect)`
- **Depends on:** config types
- **Used by:** TUI runtime

### Terminal UI Layer

- **Purpose:** Rendering and event handling
- **Location:** `pkgs/artifacts/src/tui/`
- **Contains:**
  - `views/` - Render functions for each screen
  - `runtime.rs` - Main event loop with effect execution
  - `effect_handler.rs` - Backend integration, script execution
  - `events.rs` - EventSource trait + implementations
  - `model_builder.rs` - Build Model from config with filtering
- **Depends on:** ratatui, crossterm, app layer, backend layer
- **Used by:** CLI module

### Backend Operations Layer

- **Purpose:** Script execution and file operations
- **Location:** `pkgs/artifacts/src/backend/`
- **Contains:**
  - `generator.rs` - Generator script execution in bubblewrap
  - `serialization.rs` - Serialize/deserialize script execution
  - `prompt.rs` - User prompt handling
  - `helpers.rs` - Utility functions
  - `tempfile.rs` - Temporary directory management
- **Depends on:** std::process, tempfile
- **Used by:** TUI effect handler

### CLI Layer

- **Purpose:** Entry point, argument parsing, orchestration
- **Location:** `pkgs/artifacts/src/cli/`
- **Contains:**
  - `mod.rs` - Main orchestration, path resolution, TUI setup
  - `args.rs` - clap argument definitions
  - `logging.rs` - Logging initialization
- **Depends on:** clap, log, all other layers
- **Used by:** `bin/artifacts.rs` entry point

## Data Flow

### Configuration Flow:

```
flake.nix ──┬──> nixosConfigurations
            │      └── artifacts.store.<name> ──> make.json
            │
            └──> homeConfigurations
                   └── artifacts.store.<name> ──> make.json

backend.toml ───> BackendConfiguration
```

### Runtime Flow (TUI):

```
1. CLI resolves paths:
   - flake.nix directory
   - backend.toml (or NIXOS_ARTIFACTS_BACKEND_CONFIG)
   - make.json (via nix eval)

2. Load configurations:
   - BackendConfiguration::read_backend_config()
   - MakeConfiguration::read_make_config()

3. Build Model:
   - Filter by --machine, --home, --artifact
   - Aggregate shared artifacts
   - Create ListEntry (Single | Shared)

4. Run TUI loop:
   - render(model) -> Frame
   - events.next_event() -> Msg
   - update(model, msg) -> (Model, Effect)
   - effect_handler.execute(effect) -> Vec<Msg>
   - Repeat

5. Effects trigger backend scripts:
   - CheckSerialization: check_serialization script
   - RunGenerator: generator script in bubblewrap
   - Serialize: serialize script in bubblewrap
```

### Artifact Generation Flow:

```
User Input
    │
    ▼
┌─────────────┐
│  Prompt UI  │──> Collect values ──> $prompts/<name>
└─────────────┘
    │
    ▼
┌─────────────┐
│  Generator  │──> Run in bubblewrap
└─────────────┘       - $prompts dir mounted
    │                 - $out dir for output
    ▼                 - $config JSON file
┌─────────────┐
│   Verify    │──> Check all expected files exist
└─────────────┘
    │
    ▼
┌─────────────┐
│  Serialize  │──> Run serialize script
└─────────────┘       - $out dir mounted
                      - Backend stores to agenix/sops/etc
```

## Key Abstractions

### Artifact

- **Purpose:** Named bundle of files produced by a generator
- **Definition:** `ArtifactDef` in `src/config/make.rs`
- **Pattern:** Declarative in Nix, instantiated in Rust
- **Fields:** name, shared (bool), files (map), prompts (map), generator (path),
  serialization (backend name)

### Backend

- **Purpose:** Pluggable storage engine with serialize/deserialize/check
  operations
- **Definition:** `BackendEntry` in `src/config/backend.rs`
- **Pattern:** Script-based plugin system via TOML configuration
- **Scripts:** check_serialization, serialize, deserialize (per target type:
  nixos, home, shared)

### Shared Artifact

- **Purpose:** Artifact generated once, distributed to multiple targets
- **Definition:** `SharedArtifactInfo` in `src/config/make.rs`
- **Pattern:** Aggregation across targets with generator selection
- **Key feature:** Multiple generators possible, user selects which to run

### Elm Architecture

- **Model:** Immutable application state
- **Msg:** Events that trigger state transitions
- **Update:** Pure function `(Model, Msg) -> (Model, Effect)`
- **Effect:** Description of side effect to execute
- **View:** Pure render function `&Model -> Frame`

### Target Types

- **Nixos:** Full NixOS machine configuration with owner/group permissions
- **HomeManager:** User-specific configuration without system permissions

## Entry Points

### CLI Entry

- **Location:** `pkgs/artifacts/src/bin/artifacts.rs`
- **Triggers:** User runs `artifacts` command
- **Responsibilities:** Initialize logging, call CLI module, handle errors

### CLI Orchestration

- **Location:** `pkgs/artifacts/src/cli/mod.rs` → `run()`
- **Triggers:** Entry point calls `artifacts::cli::run()`
- **Responsibilities:** Parse args, resolve paths, load configs, run TUI, report
  failures

### TUI Runtime

- **Location:** `pkgs/artifacts/src/tui/runtime.rs` → `run()`
- **Triggers:** CLI calls `run_tui_loop()`
- **Responsibilities:** Event loop, effect execution, rendering coordination

### NixOS Module

- **Location:** `modules/default.nix` → `artifacts.store.<name>`
- **Triggers:** NixOS evaluation
- **Responsibilities:** Define option schema, generate make.json via Nix

## Error Handling

**Strategy:** `anyhow::Result<T>` throughout, with context via `.with_context()`

**Patterns:**

- Early returns with `?` operator
- Context messages at layer boundaries
- Script errors captured from stderr
- Non-blocking warnings for backend capability issues

## Cross-Cutting Concerns

**Logging:** `log` crate with emoji support, levels: Error, Warning, Info,
Debug, Trace

**Validation:**

- Backend script presence validation at config load
- File generation verification before serialization
- Backend capability validation (shared support)

**Security:**

- Bubblewrap containers for generator/serialize scripts
- Temporary directories with restricted permissions
- No network access in sandbox (by default)

---

_Architecture analysis: 2025-02-13_
