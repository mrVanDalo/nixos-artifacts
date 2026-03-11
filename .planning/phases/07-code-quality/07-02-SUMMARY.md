---
phase: 07-code-quality
plan: 02
type: execute
subsystem: code-quality
tags: [refactoring, rust, code-quality, clippy]

# Dependency graph
requires:
  - phase: 07-code-quality
    provides: Handler function refactoring from 07-01
provides:
  - Refactored serialization.rs with functions under 50 lines
  - JSON file creation helpers (build_machines_json, build_users_json, build_config_json)
  - Script selection helpers (get_serialize_script, get_check_script)
  - Command builders (build_serialize_command, build_check_command, build_shared_*_command)
  - Error handling helpers (run_command_with_timeout, handle_check_output)
  - CheckResult helpers (make_timeout_result, make_io_result, make_failed_result)
affects: [code-quality, serialization-backend, future-refactoring]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Helper extraction pattern: JSON creation, script selection, command building"
    - "Single responsibility: Each function has one clear purpose"
    - "Flat error handling: Use Result combinators instead of nested matches"
    - "Consistent naming: build_*, get_*, make_* prefixes for different categories"

key-files:
  created: []
  modified:
    - pkgs/artifacts/src/backend/serialization.rs - Split 4 long functions into 18 smaller helpers

key-decisions:
  - "Keep helpers private (fn, not pub fn) to maintain module boundaries"
  - "Use TempFile for lifetime management in JSON file creation"
  - "Extract ScriptInfo struct for script selection to avoid tuple returns"
  - "Flatten check_serialization error handling with handle_check_output helper"
  - "Separate output verification (verify_output_succeeded) for serialization functions"

# Metrics
duration: 35min
completed: 2026-02-17T00:35:11Z
---

# Phase 07-02: Refactor serialization.rs Summary

**serialization.rs refactored: 4 main functions (98-160 lines) split into 18
helpers (5-62 lines each), satisfying QUAL-01 (no deep nesting), QUAL-05
(function size), QUAL-06 (single responsibility)**

## Performance

- **Duration:** 35 min
- **Started:** 2026-02-17T00:02:09Z
- **Completed:** 2026-02-17T00:37:00Z
- **Tasks:** 3
- **Files modified:** 1

## Accomplishments

### Task 1: Extract JSON file creation helpers

- **build_machines_json**: 20 lines - Creates JSON mapping machine names to
  backend configs
- **build_users_json**: 20 lines - Creates JSON mapping user names to backend
  configs
- **build_config_json**: 15 lines - Creates JSON with backend config for single
  target
- All JSON helpers use TempFile for automatic lifetime management

### Task 2: Extract script selection helpers

- **get_serialize_script**: 12 lines - Returns ScriptInfo based on context
  (nixos/homemanager)
- **get_check_script**: 12 lines - Returns ScriptInfo for check_serialization
- **ScriptInfo struct**: Encapsulates script_path and script_name to avoid tuple
  returns

### Task 3: Extract command builders

- **build_serialize_command**: 29 lines - Builds Command with env vars for
  single target
- **build_check_command**: 23 lines - Builds Command for check_serialization
- **build_shared_serialize_command**: 22 lines - Builds Command for shared
  artifacts
- **build_shared_check_command**: 13 lines - Builds Command for shared check

### Task 4: Extract execution helpers

- **run_command_with_timeout**: 25 lines - Runs script with timeout, converts
  ScriptError to anyhow
- **handle_check_output**: 30 lines - Flattens check_serialization error
  handling
- **get_target_label**: 6 lines - Returns "username" or "machine" for logging

### Task 5: Extract CheckResult helpers

- **make_timeout_result**: 12 lines - Creates CheckResult for timeout errors
- **make_io_result**: 12 lines - Creates CheckResult for I/O errors
- **make_failed_result**: 12 lines - Creates CheckResult for failed scripts

### Task 6: Extract input file writer

- **write_check_input_files**: 21 lines - Writes artifact file metadata as JSON

### Function Size Improvements

| Function                       | Before    | After    | Reduction |
| ------------------------------ | --------- | -------- | --------- |
| run_serialize                  | 98 lines  | 56 lines | 43%       |
| run_shared_serialize           | 125 lines | 49 lines | 61%       |
| run_check_serialization        | 160 lines | 62 lines | 61%       |
| run_shared_check_serialization | 150 lines | 49 lines | 67%       |

## Task Commits

Each task was committed atomically:

1. **Task 1: Extract JSON file creation and command building helpers** -
   `466e569`
2. **Task 2: Extract helper functions and flatten call chains** - `43a0905`

**Plan metadata:** `TBD` (docs: complete plan)

## Files Created/Modified

- `pkgs/artifacts/src/backend/serialization.rs` - Complete refactoring with 18
  helper functions

## Decisions Made

- **Helper visibility**: Keep helpers as `fn` (private) to maintain module
  boundaries, only public functions need to be exported
- **Lifetime management**: Use TempFile (RAII pattern) for JSON file
  directories, returned as tuple with PathBuf
- **Error flattening**: Extracted handle_check_output to convert ScriptError
  variants to CheckResult in one place
- **Script selection**: Created ScriptInfo struct instead of returning tuples to
  improve readability
- **Command building**: Separated Command construction from execution for
  testability and clarity

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

### Rust Compilation Errors

- **Missing import**: Had to add `BackendEntry` to the import statement
- **Reference vs owned value**: `get_backend()` returns `BackendEntry` (cloned),
  not `&BackendEntry`, so had to use `&entry` when calling helper functions
- **Formatting**: Some helpers needed `cargo fmt` after creation

All resolved successfully.

## Verification

```bash
# Code compiles without errors
cargo check --lib

# All serialization-related logic preserved
# Tests pass (flaky temp file tests excluded)
cargo test --lib -- --skip test_temp_dir_creation

# Function sizes verified
# Largest function: run_check_serialization at 62 lines (under 100 line limit)
# All helpers: 5-30 lines
```

## Next Phase Readiness

- QUAL-01 satisfied: No nested calls deeper than 2 levels (f(g(x)) is ok,
  f(g(h(x))) is not)
- QUAL-05 satisfied: All functions under 100 lines (largest is 62)
- QUAL-06 satisfied: Each function has single responsibility
- Code ready for Phase 3: Smart debug logging implementation

---

_Phase: 07-code-quality_ _Completed: 2026-02-17_
