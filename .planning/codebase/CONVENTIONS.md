# Coding Conventions

**Analysis Date:** 2025-02-13

## Language & Framework

**Primary Language:** Rust (1.87.0) **Framework:** CLI/TUI application using
ratatui for terminal UI **Edition:** 2024

## Naming Patterns

**Files:**

- Module files: `snake_case.rs` (e.g., `effect_handler.rs`, `model_builder.rs`)
- Test files: Located in `tests/` directory with matching module structure

**Functions/Methods:**

- Use descriptive, non-abbreviated names
- Prefer `snake_case` for functions
- Constructor functions: `new()`, `default()`
- Builder pattern methods: `with_` prefix (e.g., `with_backend()`)

**Variables:**

- Use descriptive names, no abbreviations
- Iterators: `entry`, `item`, `artifact` (singular forms)
- Collections: Plural forms (e.g., `artifacts`, `entries`, `generators`)
- Mutable references: `ref` prefix avoided, use `_mut` suffix for mut getters
  (e.g., `get_mut()`)

**Types:**

- Structs/Enums: `PascalCase` with descriptive names (e.g., `ArtifactStatus`,
  `BackendConfiguration`)
- Type aliases: Rarely used; prefer newtype pattern
- Associated types: Clear naming that indicates purpose (e.g., `EventSource`,
  `EffectHandler`)

**Constants:**

- `UPPER_SNAKE_CASE` for true constants
- Associated constants on types preferred over module-level constants

**Modules:**

- Directory modules have `mod.rs` as entry point
- Flat module structure preferred when appropriate

## Code Style

**Formatting:**

- Standard `rustfmt` formatting
- Max line length: 100 characters (implied from code)
- Trailing commas in multi-line collections

**Linting:**

- Clippy: Used with default settings (warnings treated as errors in CI)
- Run: `cargo clippy`

## Import Organization

**Order:**

1. Standard library imports (`std::`)
2. Third-party crate imports (`anyhow::`, `serde::`, `ratatui::`, etc.)
3. Internal crate imports (`crate::`)

**Pattern:**

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::app::message::{KeyEvent, Msg};
use crate::config::make::MakeConfiguration;
```

**Path Aliases:**

- No custom path aliases configured
- Use absolute imports starting with `crate::`

## Error Handling

**Pattern:**

- Primary error type: `anyhow::Result<T>` for application errors
- Context propagation: Use `.with_context()` for adding error context
- Early returns: `?` operator preferred
- Explicit error construction: `anyhow::bail!()` for fail-fast errors

**Example:**

```rust
pub fn read_backend_config(backend_toml: &Path) -> Result<BackendConfiguration> {
    let text = fs::read_to_string(&canonical)
        .with_context(|| format!("reading backend config {}", toml_path.display()))?;
    
    if !condition {
        anyhow::bail!("backend '{}' requires script", name);
    }
}
```

**Error Messages:**

- Include relevant context (file paths, backend names, step names)
- Clear, actionable error descriptions
- Use `display()` for path formatting in error messages

## Logging

**Framework:** `log` crate with structured logging **Patterns:**

- Error level: Fatal errors that cause program exit
- Warning level: Non-blocking issues (e.g., backend capability mismatches)
- Info level: User-facing progress messages
- Debug/Trace: Detailed execution information

**Entry Point Error Handling:**

```rust
fn main() {
    if let Err(err) = artifacts::cli::run() {
        error!("{:#}", err);
        std::process::exit(1);
    }
}
```

## Comments

**Style:**

- Doc comments (`///`) for public items
- Implementation comments (`//`) for complex logic
- Module-level documentation (`//!`) at top of files

**When to Comment:**

- Complex business logic (e.g., TUI state machine transitions)
- Architecture patterns (e.g., Elm Architecture explanation)
- Workarounds or non-obvious code
- Safety justifications (e.g., `// SAFETY: Tests run sequentially`)

**Documentation:**

- All public functions, structs, enums have doc comments
- Include examples in doc comments where helpful
- Document state machine transitions and invariants

## Function Design

**Size:**

- Keep functions small and focused
- Break long functions into smaller, sequential functions
- Pure functions preferred where possible

**Parameters:**

- Use references (`&Path`, `&str`) for input parameters
- Use owned types for configuration/builder patterns
- Builder pattern for complex initialization

**Return Values:**

- Return `Result<T>` for fallible operations
- Return `Option<T>` for potentially missing values
- Use tuple destructuring for multiple returns

**Pure Functions:**

- Elm Architecture pattern: Update functions are pure
  `(Model, Msg) -> (Model, Effect)`
- View functions are pure `&Model -> Frame`

## Module Design

**Exports:**

- Public modules re-export key types at module level
- `pub use` for commonly used items
- Private modules use `mod` without `pub`

**Pattern (src/app/mod.rs):**

```rust
pub mod effect;
pub mod message;
pub mod model;
pub mod update;

pub use effect::Effect;
pub use message::{KeyEvent, Msg};
pub use model::{ArtifactEntry, ArtifactStatus, Model, Screen};
pub use update::{init, update};
```

**Barrel Files:**

- Each module has `mod.rs` that exports public interface
- Re-export to reduce import verbosity

## Architecture Patterns

**Elm Architecture (TUI):**

- **Model**: Application state (immutable)
- **Msg**: Events/actions
- **Update**: Pure function `(Model, Msg) -> (Model, Effect)`
- **Effect**: Side effect descriptors (not execution)
- **View**: Pure rendering function

**Effect Pattern:**

```rust
pub enum Effect {
    None,
    CheckSerialization { ... },
    RunGenerator { ... },
    Serialize { ... },
}
```

**Trait Abstractions:**

- `EventSource` trait for testable event input
- `EffectHandler` trait for testable side effects

## Coding Principles

1. **Fail fast** - Return errors early, don't continue with invalid state
2. **No abbreviations** - Use clear, descriptive names
3. **Function size** - Break long functions into smaller, sequential functions
4. **Immutability** - Prefer immutable data structures
5. **Type safety** - Leverage Rust's type system (enums for state machines)

## Serialization

**Format:** TOML for configuration, JSON for data exchange **Pattern:**

- Derive `Serialize`/`Deserialize` for config types
- Use `serde(default)` for optional fields
- Flatten nested structures where appropriate

---

_Convention analysis: 2025-02-13_
