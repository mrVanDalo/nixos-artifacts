# Plan 06: Unified Backend Environment Variables

## Overview

This plan unifies the environment variables passed to backend scripts so that a single script can handle all contexts (NixOS, HomeManager, Shared) without complex conditional logic. This is a **breaking change** that removes the legacy per-context variables.

## Current State (Problems)

### Single Target Environment
```bash
$artifact          # Artifact name
$artifact_context  # "nixos" or "homemanager"
$config            # Path to config.json (flat backend settings)
$machine           # Machine name (NixOS only)
$username          # Username (HomeManager only)
$out               # Output directory (serialize only)
$inputs            # Inputs directory (check only)
$LOG_LEVEL         # Log level
```

### Shared Target Environment
```bash
$artifact          # Artifact name
# $artifact_context NOT SET (problem!)
# $config NOT SET (problem!)
$machines          # Path to machines.json
$users             # Path to users.json
# $inputs NOT SET for check (problem!)
$out               # Output directory (serialize only)
$LOG_LEVEL         # Log level
```

### Problems
1. `$artifact_context` not set for shared - script can't distinguish context
2. `$config` not set for shared - no unified config access
3. `$inputs` not set for shared check - inconsistent check behavior
4. Different JSON structures - single has flat config, shared has nested per-target
5. No target list for single - can't write one script that iterates

## Target State (Unified)

### Unified Environment Variables (All Contexts)

| Variable | Value | When Set |
|----------|-------|----------|
| `$artifact` | Artifact name | Always |
| `$artifact_context` | `"nixos"`, `"homemanager"`, or `"shared"` | Always |
| `$targets` | Path to `targets.json` | Always |
| `$out` | Output directory | Serialize only |
| `$inputs` | Inputs directory | Check only |
| `$LOG_LEVEL` | Log level | Always |

### Removed Variables (Breaking Change)
- `$config` - replaced by `$targets`
- `$machines` - replaced by `$targets`
- `$users` - replaced by `$targets`
- `$machine` - available in `$targets` JSON
- `$username` - available in `$targets` JSON

### Unified `targets.json` Structure

For single NixOS target:
```json
{
  "context": "nixos",
  "targets": [
    {
      "name": "machine-one",
      "type": "nixos",
      "config": { "key": "value" }
    }
  ]
}
```

For single HomeManager target:
```json
{
  "context": "homemanager",
  "targets": [
    {
      "name": "alice",
      "type": "homemanager",
      "config": { "key": "value" }
    }
  ]
}
```

For shared artifact:
```json
{
  "context": "shared",
  "targets": [
    {
      "name": "machine-one",
      "type": "nixos",
      "config": { "key": "value" }
    },
    {
      "name": "machine-two",
      "type": "nixos",
      "config": { "key": "value" }
    },
    {
      "name": "alice",
      "type": "homemanager",
      "config": { "key": "value" }
    }
  ]
}
```

### Unified `inputs/` Directory Structure (Check Only)

For all contexts, `$inputs` contains one JSON file per expected output file:

```
$inputs/
  secret-key.json      # { "path": "/path/to/secret-key", "owner": "root", "group": "root" }
  other-file.json      # { "path": "/path/to/other-file", "owner": "user", "group": "users" }
```

This is already the structure for single targets; we extend it to shared.

## Implementation Phases

### Phase 1: Update `SerializationContext` and Config Building

**File:** `pkgs/artifacts/src/backend/serialization.rs`

1. Remove `ConfigPaths` enum variants, replace with unified structure:

```rust
struct ConfigFiles {
    _handles: Vec<TempFile>,
    targets_path: PathBuf,
    inputs_path: Option<PathBuf>,  // Set for check operations
}
```

2. Add `build_targets_json()` function:

```rust
fn build_targets_json(
    ctx: &SerializationContext<'_>,
    make: &MakeConfiguration,
) -> Result<(TempFile, PathBuf)> {
    let dir = TempFile::new_dir("targets")?;
    let path = dir.join("targets.json");

    let (context_str, targets) = match ctx {
        SerializationContext::Single { artifact, target_type } => {
            let target_name = target_type.target_name();
            let type_str = target_type.context_str();
            let config = make
                .get_backend_config_for(target_name, &artifact.serialization)
                .map(|m| serde_json::to_value(m).unwrap_or(json!({})))
                .unwrap_or(json!({}));
            (
                type_str.to_string(),
                vec![json!({
                    "name": target_name,
                    "type": type_str,
                    "config": config
                })]
            )
        }
        SerializationContext::Shared { backend_name, nixos_targets, home_targets, .. } => {
            let mut targets = Vec::new();
            for machine in nixos_targets.iter() {
                let config = make
                    .get_backend_config_for(machine, backend_name)
                    .map(|m| serde_json::to_value(m).unwrap_or(json!({})))
                    .unwrap_or(json!({}));
                targets.push(json!({
                    "name": machine,
                    "type": "nixos",
                    "config": config
                }));
            }
            for user in home_targets.iter() {
                let config = make
                    .get_backend_config_for(user, backend_name)
                    .map(|m| serde_json::to_value(m).unwrap_or(json!({})))
                    .unwrap_or(json!({}));
                targets.push(json!({
                    "name": user,
                    "type": "homemanager",
                    "config": config
                }));
            }
            ("shared".to_string(), targets)
        }
    };

    let json = json!({
        "context": context_str,
        "targets": targets
    });

    let text = to_string_pretty(&json)?;
    fs::write(&path, &text)?;
    Ok((dir, path))
}
```

3. Remove these functions (no longer needed):
   - `build_config_json()`
   - `build_machines_json()`
   - `build_users_json()`

4. Update `SerializationContext::build_config_files()` to use unified builder:

```rust
fn build_config_files(&self, make: &MakeConfiguration) -> Result<ConfigFiles> {
    let (targets_handle, targets_path) = build_targets_json(self, make)?;
    Ok(ConfigFiles {
        _handles: vec![targets_handle],
        targets_path,
        inputs_path: None,  // Set later for check operations
    })
}
```

### Phase 2: Update `apply_env()` Method

**File:** `pkgs/artifacts/src/backend/serialization.rs`

Replace the current `apply_env()` with simplified version:

```rust
fn apply_env(&self, cmd: &mut Command, config: &ConfigFiles) {
    // Always set unified variables
    cmd.env("artifact", self.artifact_name());
    cmd.env("artifact_context", self.context_str());
    cmd.env("targets", &config.targets_path);

    // Set inputs if available (check operations)
    if let Some(ref inputs_path) = config.inputs_path {
        cmd.env("inputs", inputs_path);
    }
}
```

Add helper method to `SerializationContext`:

```rust
fn context_str(&self) -> &'static str {
    match self {
        SerializationContext::Single { target_type, .. } => target_type.context_str(),
        SerializationContext::Shared { .. } => "shared",
    }
}
```

### Phase 3: Update Check Operations for Shared Artifacts

**File:** `pkgs/artifacts/src/backend/serialization.rs`

Currently `run_check_inner` only creates `$inputs` for single targets. Update to create inputs for shared too.

1. Add function to write inputs for shared artifacts:

```rust
fn write_shared_check_input_files(
    artifact_name: &str,
    make: &MakeConfiguration,
    inputs_dir: &Path,
) -> Result<()> {
    // Get file definitions from the shared artifact info
    if let Some(shared_info) = make.get_shared_artifacts().get(artifact_name) {
        for file in shared_info.files.values() {
            let resolved_path = file
                .path
                .as_ref()
                .map(|path| resolve_path(&make.make_base, path));
            let json_path = inputs_dir.join(&file.name);

            let text = to_string_pretty(&json!({
                "path": resolved_path,
                "owner": file.owner,
                "group": file.group,
            }))?;

            fs::write(&json_path, text)?;
        }
    }
    Ok(())
}
```

2. Update `run_check_inner` to always create inputs:

```rust
fn run_check_inner(
    ctx: &SerializationContext<'_>,
    backend: &BackendConfiguration,
    make: &MakeConfiguration,
    log_level: crate::logging::LogLevel,
) -> Result<CheckResult> {
    // ... existing setup code ...

    // Create inputs directory for ALL contexts
    let inputs = TempFile::new_dir_with_name(&format!("inputs-{}", ctx.artifact_name()))?;

    match ctx {
        SerializationContext::Single { artifact, .. } => {
            write_check_input_files(artifact, &inputs, make)?;
        }
        SerializationContext::Shared { artifact_name, .. } => {
            write_shared_check_input_files(artifact_name, make, &inputs)?;
        }
    }

    let mut config = ctx.build_config_files(make)?;
    config.inputs_path = Some(inputs.as_ref().to_path_buf());

    // ... rest of function ...
}
```

### Phase 4: Update Public API Functions

**File:** `pkgs/artifacts/src/backend/serialization.rs`

Update function signatures to remove the inputs_dir parameter from `run_check_inner`:

```rust
// OLD signature
fn run_check_inner(
    ctx: &SerializationContext<'_>,
    backend: &BackendConfiguration,
    make: &MakeConfiguration,
    inputs_dir: Option<&TempFile>,  // REMOVE THIS
    log_level: crate::logging::LogLevel,
) -> Result<CheckResult>

// NEW signature
fn run_check_inner(
    ctx: &SerializationContext<'_>,
    backend: &BackendConfiguration,
    make: &MakeConfiguration,
    log_level: crate::logging::LogLevel,
) -> Result<CheckResult>
```

Update `run_check_serialization`:

```rust
pub fn run_check_serialization(
    artifact: &ArtifactDef,
    target_type: &TargetType,
    backend: &BackendConfiguration,
    make: &MakeConfiguration,
    log_level: crate::logging::LogLevel,
) -> Result<CheckResult> {
    let ctx = SerializationContext::Single {
        artifact,
        target_type,
    };
    run_check_inner(&ctx, backend, make, log_level)
}
```

Update `run_shared_check_serialization`:

```rust
pub fn run_shared_check_serialization(
    artifact_name: &str,
    backend_name: &str,
    backend: &BackendConfiguration,
    make: &MakeConfiguration,
    nixos_targets: &[String],
    home_targets: &[String],
    log_level: crate::logging::LogLevel,
) -> Result<CheckResult> {
    let ctx = SerializationContext::Shared {
        artifact_name,
        backend_name,
        nixos_targets,
        home_targets,
    };
    run_check_inner(&ctx, backend, make, log_level)
}
```

### Phase 5: Update Logging Methods

**File:** `pkgs/artifacts/src/backend/serialization.rs`

Simplify `log_serialize_env` and `log_check_env`:

```rust
fn log_serialize_env(
    &self,
    script_name: &str,
    script_path: &Path,
    out: &Path,
    config: &ConfigFiles,
) {
    log_debug!(
        "running {}: script=\"{}\"",
        script_name,
        script_path.display()
    );
    log_debug!(
        "  environment: out=\"{}\" targets=\"{}\" artifact=\"{}\" artifact_context=\"{}\"",
        out.display(),
        config.targets_path.display(),
        self.artifact_name(),
        self.context_str()
    );
}

fn log_check_env(
    &self,
    script_name: &str,
    script_path: &Path,
    config: &ConfigFiles,
) {
    log_debug!(
        "running {}: script=\"{}\"",
        script_name,
        script_path.display()
    );
    let inputs_str = config.inputs_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<none>".to_string());
    log_debug!(
        "  environment: inputs=\"{}\" targets=\"{}\" artifact=\"{}\" artifact_context=\"{}\"",
        inputs_str,
        config.targets_path.display(),
        self.artifact_name(),
        self.context_str()
    );
}
```

### Phase 6: Update Test Backend Scripts

**Files:**
- `pkgs/artifacts/examples/backends/test-config-verify/check.sh`
- `pkgs/artifacts/examples/backends/test-config-verify/serialize.sh`
- `pkgs/artifacts/examples/backends/test-config-verify/shared_check.sh`
- `pkgs/artifacts/examples/backends/test-config-verify/shared_serialize.sh`

Since the interface is now unified, we can simplify to fewer scripts. But first, update each to use the new variables:

**Unified `check.sh`:**
```bash
#!/usr/bin/env bash
# Unified check script for all contexts (nixos, homemanager, shared)

# Verify required environment variables
if [ -z "$artifact" ]; then
    echo "ERROR: \$artifact is not set" >&2
    exit 1
fi

if [ -z "$artifact_context" ]; then
    echo "ERROR: \$artifact_context is not set" >&2
    exit 1
fi

if [ -z "$targets" ]; then
    echo "ERROR: \$targets is not set" >&2
    exit 1
fi

if [ ! -f "$targets" ]; then
    echo "ERROR: \$targets file does not exist: $targets" >&2
    exit 1
fi

if [ -z "$inputs" ]; then
    echo "ERROR: \$inputs is not set" >&2
    exit 1
fi

if [ ! -d "$inputs" ]; then
    echo "ERROR: \$inputs directory does not exist: $inputs" >&2
    exit 1
fi

# Output for snapshot verification
echo "# BEGIN CHECK SNAPSHOT"
echo "# artifact=$artifact"
echo "# artifact_context=$artifact_context"
echo "# TARGETS FILE:"
cat "$targets"
echo "# INPUTS DIRECTORY:"
ls -la "$inputs"
echo "# END CHECK SNAPSHOT"

# Always request generation (for test purposes)
exit 1
```

**Unified `serialize.sh`:**
```bash
#!/usr/bin/env bash
# Unified serialize script for all contexts (nixos, homemanager, shared)

# Verify required environment variables
if [ -z "$artifact" ]; then
    echo "ERROR: \$artifact is not set" >&2
    exit 1
fi

if [ -z "$artifact_context" ]; then
    echo "ERROR: \$artifact_context is not set" >&2
    exit 1
fi

if [ -z "$targets" ]; then
    echo "ERROR: \$targets is not set" >&2
    exit 1
fi

if [ ! -f "$targets" ]; then
    echo "ERROR: \$targets file does not exist: $targets" >&2
    exit 1
fi

if [ -z "$out" ]; then
    echo "ERROR: \$out is not set" >&2
    exit 1
fi

if [ ! -d "$out" ]; then
    echo "ERROR: \$out directory does not exist: $out" >&2
    exit 1
fi

# Output for snapshot verification
echo "# BEGIN SERIALIZE SNAPSHOT"
echo "# artifact=$artifact"
echo "# artifact_context=$artifact_context"
echo "# TARGETS FILE:"
cat "$targets"
echo "# OUTPUT DIRECTORY:"
ls -la "$out"
echo "# END SERIALIZE SNAPSHOT"

exit 0
```

### Phase 7: Update Example Scenario Scripts

**Files in `pkgs/artifacts/examples/scenarios/*/`:**

Update all `test_serialize.sh` and similar scripts to use `$targets` instead of `$config`, `$machines`, `$users`.

Key files to update:
- `single-artifact-with-prompts/test_serialize.sh`
- `two-artifacts-no-prompts/test_serialize.sh`
- `home-manager-only/test_serialize.sh`
- `home-manager/test_serialize.sh`
- `multiple-machines/test_serialize.sh`
- `no-config-section/test_serialize.sh`
- `shared-artifacts/test_serialize.sh`
- `backend-include/backend_test/serialize.sh`
- `artifact-name-formats/test_serialize.sh`

### Phase 8: Update Test Backend in backends/test/

**File:** `backends/test/default.nix`

The test backend currently defines separate scripts. Update to use unified scripts:

```nix
{
  perSystem =
    { pkgs, self', ... }:
    {
      packages.test-backend = self'.lib.mkBackend {
        system = pkgs.system;
        name = "test";
        # All targets use the same unified scripts
        nixos_check = ./check.sh;
        nixos_serialize = ./serialize.sh;
        home_check = ./check.sh;
        home_serialize = ./serialize.sh;
        shared_check = ./check.sh;
        shared_serialize = ./serialize.sh;
      };
    };
}
```

**File:** `backends/test/check.sh` (unified)
**File:** `backends/test/serialize.sh` (unified)

### Phase 9: Update E2E Tests

**File:** `pkgs/artifacts/tests/e2e/config_env_tests.rs`

Update test names and assertions to reflect unified interface:

```rust
// Rename tests to reflect unified behavior
#[test]
fn e2e_unified_check_sets_targets() -> Result<()> { ... }

#[test]
fn e2e_unified_serialize_sets_targets() -> Result<()> { ... }

#[test]
fn e2e_unified_shared_check_sets_targets() -> Result<()> { ... }

#[test]
fn e2e_unified_shared_serialize_sets_targets() -> Result<()> { ... }
```

### Phase 10: Regenerate Snapshots

Run tests and update snapshots:

```bash
cd pkgs/artifacts
cargo test --test tests e2e_config -- --ignored
cargo insta review
```

### Phase 11: Update Documentation

**Files to update:**
- `docs/modules/ROOT/pages/backend-scripts-reference.adoc`
- `docs/modules/ROOT/pages/backend-quickstart.adoc`
- `docs/modules/ROOT/pages/reference-mkbackend.adoc`
- `BACKEND_GUIDE.md`
- `pkgs/artifacts/CLAUDE.md`

Document the new unified interface and remove references to legacy variables.

## Files Changed Summary

| Phase | Files | Lines Changed (Est.) |
|-------|-------|---------------------|
| 1-5 | `src/backend/serialization.rs` | ~200 removed, ~150 added |
| 6 | `examples/backends/test-config-verify/*.sh` | ~100 rewritten |
| 7 | `examples/scenarios/*/*.sh` | ~50 updated |
| 8 | `backends/test/*.sh` | ~40 rewritten |
| 9 | `tests/e2e/config_env_tests.rs` | ~30 updated |
| 10 | `tests/e2e/snapshots/*.snap` | ~4 regenerated |
| 11 | Documentation | ~300 updated |

**Net change:** ~-50 lines in Rust, ~100 lines simplified in shell scripts

## Verification Steps

After each phase:
1. `cargo check` - compilation
2. `cargo clippy` - no warnings
3. `cargo test --lib` - unit tests

After all phases:
4. `cargo test --test tests` - integration tests
5. `nix flake check` - flake validation
6. `nix fmt` - formatting

## Migration Notes for Backend Authors

**Before (multiple scripts needed):**
```bash
# check.sh - must handle $config, $machine OR $username
# shared_check.sh - must handle $machines, $users (different JSON!)
```

**After (one script handles all):**
```bash
#!/usr/bin/env bash
# Works for nixos, homemanager, AND shared

# Parse targets.json with jq
context=$(jq -r '.context' "$targets")
num_targets=$(jq '.targets | length' "$targets")

echo "Context: $context, Targets: $num_targets"

# Iterate over all targets uniformly
jq -c '.targets[]' "$targets" | while read -r target; do
    name=$(echo "$target" | jq -r '.name')
    type=$(echo "$target" | jq -r '.type')
    echo "Processing $type target: $name"
done
```

## Dependencies

- Plan 05 (TargetSpec unification) - Completed ✅
- This builds on the Rust-side Effect/Message unification
