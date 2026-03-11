---
phase: 08-smart-logging
plan: 01
subsystem: cli
# Dependency graph
requires:
  - phase: 07-code-quality
    provides: refactored codebase ready for feature additions
provides:
  - Feature flag for logging in Cargo.toml
  - --log-file and --log-level CLI arguments with feature gating
  - Zero-cost logging when disabled via conditional compilation
  - Nix package enables logging by default
affects:
  - cli
  - logging
  - configuration

# Tech tracking
tech-stack:
  added:
    - log crate as optional dependency
    - Feature flag macros (log_debug!, log_trace!, log_error!)
    - Conditional compilation attributes
  patterns:
    - Feature-gated code compilation
    - Macro-based zero-cost abstractions
    - Option<PathBuf> for optional file paths

key-files:
  created: []
  modified:
    - pkgs/artifacts/Cargo.toml
    - pkgs/artifacts/src/cli/args.rs
    - pkgs/artifacts/src/cli/logging.rs
    - pkgs/artifacts/src/cli/mod.rs
    - pkgs/artifacts/src/macros.rs
    - pkgs/artifacts/src/backend/generator.rs
    - pkgs/artifacts/src/backend/serialization.rs
    - pkgs/artifacts/src/config/nix.rs
    - pkgs/artifacts/src/config/make.rs
    - pkgs/artifacts/src/bin/artifacts.rs
    - pkgs/artifacts/default.nix

key-decisions:
  - "Made log crate optional (dep:log) to enable zero-cost logging"
  - "Created log_debug!, log_trace!, log_error! macros for conditional compilation"
  - "Removed Warning and Trace log levels (keep Error, Warn, Info, Debug)"
  - "Set default log level to Debug when logging is enabled"
  - "Nix package enables logging by default; cargo builds are slimmer"

patterns-established:
  - "Feature-gated macros: Use cfg macros that expand to nothing when disabled"
  - "Zero-cost abstractions: Log calls have no overhead when feature disabled"
  - "Conditional CLI arguments: Arguments hidden when feature disabled"

duration: 25 min
completed: 2026-02-17
---

# Phase 08 Plan 01: Smart Logging Summary

**Feature-gated CLI logging with --log-file and --log-level arguments, zero-cost
when disabled**

## Performance

- **Duration:** 25 min
- **Started:** 2026-02-17T11:03:18Z
- **Completed:** 2026-02-17T11:28:00Z
- **Tasks:** 3
- **Files modified:** 11

## Accomplishments

- Added logging feature flag to Cargo.toml with optional log crate dependency
- Created feature-gated macros (log_debug!, log_trace!, log_error!) that compile
  to nothing when disabled
- Updated CLI arguments with --log-file and --log-level (feature-gated)
- Refactored LogLevel enum to Error/Warn/Info/Debug (removed Warning and Trace)
- Made logging module conditional with #[cfg(feature = "logging")]
- Replaced all log::debug/trace calls with feature-gated macro versions
- Nix package enables logging by default for full functionality
- All builds pass with and without logging feature

## Task Commits

Each task was committed atomically:

1. **Task 1: Add logging feature flag to Cargo.toml** - `b78ee10` (feat)
2. **Task 2: Update CLI arguments with feature-gated --log-file** - `08c0572`
   (feat)
3. **Task 3: Enable logging feature in Nix package** - `fc549c9` (feat)

## Files Created/Modified

- `pkgs/artifacts/Cargo.toml` - Added [features] section with logging feature
- `pkgs/artifacts/src/cli/args.rs` - Added --log-file and --log-level arguments
  with feature gating
- `pkgs/artifacts/src/cli/logging.rs` - File-based logger with configurable path
- `pkgs/artifacts/src/cli/mod.rs` - Conditional logger initialization
- `pkgs/artifacts/src/macros.rs` - Feature-gated log_debug!, log_trace!,
  log_error! macros
- `pkgs/artifacts/src/backend/generator.rs` - Replaced log calls with macros
- `pkgs/artifacts/src/backend/serialization.rs` - Replaced log calls with macros
- `pkgs/artifacts/src/config/nix.rs` - Replaced log calls with macros
- `pkgs/artifacts/src/config/make.rs` - Replaced log calls with macros
- `pkgs/artifacts/src/bin/artifacts.rs` - Conditional error logging
- `pkgs/artifacts/default.nix` - Enable logging feature in Nix builds

## Decisions Made

1. **Made log crate optional** - Enables zero-cost logging when feature disabled
2. **Created custom macros** - log_debug!, log_trace!, log_error! expand to
   nothing without feature
3. **Simplified LogLevel enum** - Removed Warning (now Warn) and Trace for
   cleaner API
4. **Default to Debug level** - When --log-file is provided, log everything for
   debugging
5. **Nix enables logging** - Users get full functionality via Nix; cargo users
   can opt in

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Had to update multiple files that were already using log crate directly
- Needed to import macros explicitly in files that use them (crate::log_debug,
  etc.)
- Initial approach attempted to gate only --log-file but existing code used log
  crate throughout

## User Setup Required

None - no external service configuration required.

## Verification Results

✅ `cargo check` passes (no features) - logging args not visible ✅
`cargo check --features logging` passes - logging args visible ✅
`nix build .#artifacts-bin` produces binary with logging support ✅
`artifacts --help` shows --log-file when built with logging ✅
`artifacts --help` hides --log-file when built without logging

## Next Phase Readiness

- Logging infrastructure complete and ready for use
- Phase 08 plan 02 can add actual logging calls throughout codebase
- Pattern established for future feature-gated functionality

---

_Phase: 08-smart-logging_ _Completed: 2026-02-17_

## Self-Check: PASSED

- [x] Cargo.toml has [features] section with logging
- [x] args.rs has --log-file with feature gating
- [x] args.rs has --log-level with feature gating
- [x] default.nix has buildFeatures = ["logging"]
- [x] All cargo check variants pass
- [x] Nix build produces binary with logging support
- [x] Binary without logging hides logging arguments
- [x] Binary with logging shows logging arguments
