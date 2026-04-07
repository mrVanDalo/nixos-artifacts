# Plan 05 Implementation: End-to-End Unification

## Overview

This plan unifies Single and Shared artifact handling across the full stack, building on
the `SerializationContext` and `GeneratorContext` enums already implemented in Plans 02/03.

## Implementation Phases

### Phase 1: Introduce `TargetSpec` in effect.rs

**File:** `pkgs/artifacts/src/app/effect.rs`

Add `TargetSpec` enum that abstracts over single vs multi-target:

```rust
/// Unified target specification for all artifact operations.
#[derive(Debug, Clone)]
pub enum TargetSpec {
    /// A single target (one machine or one user)
    Single(TargetType),
    /// Multiple targets (shared artifact)
    Multi {
        nixos_targets: Vec<String>,
        home_targets: Vec<String>,
    },
}
```

Then collapse Effect variants from 6 to 3:
- `CheckSerialization` + `SharedCheckSerialization` → `CheckSerialization { target_spec }`
- `RunGenerator` + `RunSharedGenerator` → `RunGenerator { target_spec }`
- `Serialize` + `SharedSerialize` → `Serialize { target_spec }`

### Phase 2: Unify Message Variants

**File:** `pkgs/artifacts/src/app/message.rs`

Remove the 3 shared message variants:
- `SharedCheckSerializationResult` - aggregation happens in background handler
- `SharedGeneratorFinished` - identical to `GeneratorFinished`
- `SharedSerializeFinished` - aggregation happens in background handler

Keep only:
- `CheckSerializationResult { artifact_index, status, result }`
- `GeneratorFinished { artifact_index, result }`
- `SerializeFinished { artifact_index, result }`

### Phase 3: Unify Background Effect Handler

**File:** `pkgs/artifacts/src/tui/background.rs`

1. Update `execute()` match to handle unified Effect variants
2. Unify handler methods to accept `TargetSpec`:
   - `execute_check_serialization(artifact_index, artifact_name, target_spec)`
   - `execute_run_generator(artifact_index, artifact_name, target_spec, prompts)`
   - `execute_serialize(artifact_index, artifact_name, target_spec)`
3. Remove the 3 `execute_shared_*` methods
4. Aggregation logic moves INTO the unified methods (match on `TargetSpec`)

### Phase 4: Unify Update Handlers

**Files:**
- `pkgs/artifacts/src/app/update/mod.rs`
- `pkgs/artifacts/src/app/update/generating.rs`
- `pkgs/artifacts/src/app/update/init.rs`
- `pkgs/artifacts/src/app/update/prompt.rs`

Changes:
1. Remove `handle_shared_check_result` - `handle_check_result` handles both
2. Remove match arms for `SharedGeneratorFinished`, `SharedSerializeFinished`
3. Remove `handle_shared_generator_finished` and `handle_shared_serialize_finished`
4. Update `handle_generator_success` to build `Effect::Serialize` with `TargetSpec`
5. Update `init.rs` to use unified `Effect::CheckSerialization`
6. Update `prompt.rs` to use unified `Effect::RunGenerator`
7. Update `start_generation_for_selected_internal` to use unified effects

### Phase 5: Update generator_selection.rs

**File:** `pkgs/artifacts/src/app/update/generator_selection.rs`

Update to emit `Effect::RunGenerator` with `TargetSpec::Multi` instead of `Effect::RunSharedGenerator`.

### Phase 6 (Optional): Simplify mkBackend Nix Interface

**Files:**
- `backends/default.nix`
- `backends/test/default.nix`

Add unified `check` and `serialize` parameters that can serve as defaults for all target types.

## Estimated Line Changes

| File | Lines Removed | Lines Added | Net |
|------|--------------|-------------|-----|
| effect.rs | ~30 | ~25 | -5 |
| message.rs | ~20 | 0 | -20 |
| background.rs | ~400 | ~150 | -250 |
| update/mod.rs | ~60 | ~10 | -50 |
| update/generating.rs | ~180 | ~20 | -160 |
| update/init.rs | ~5 | ~10 | +5 |
| update/prompt.rs | ~5 | ~10 | +5 |
| **Total** | ~700 | ~225 | **~-475** |

## Verification Steps

After each phase:
1. `cargo check` - compilation
2. `cargo clippy` - no warnings
3. `cargo test --lib` - unit tests
4. `cargo test --test tests` - integration tests (after all phases)
5. `nix flake check` - flake validation
6. `nix fmt` - formatting

## Dependencies

- ✅ Plan 02 (SerializationContext) - Already implemented
- ✅ Plan 03 (GeneratorContext) - Already implemented

## What NOT to Change

- `ListEntry` enum (Single vs Shared) - view layer needs distinct shapes
- `SelectGeneratorState` - shared-specific UI for generator selection
- `MakeConfiguration` with `nixos_map`/`home_map` - data aggregation layer
- `SharedArtifactInfo` struct - needed for shared-specific data
