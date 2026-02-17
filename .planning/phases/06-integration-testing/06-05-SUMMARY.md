---
phase: 06-integration-testing
plan: 05
subsystem: testing
tags: [diagnostics, testing, debugging, e2e, headless]

requires:
  - phase: 06-integration-testing
    provides: E2E test infrastructure from 06-01, 06-02, 06-03, 06-04

provides:
  - DiagnosticInfo struct for capturing test failure information
  - generate_single_artifact_with_diagnostics() function
  - dump_test_diagnostics() helper for writing diagnostics to files
  - e2e/diagnostics.rs test module with 6 diagnostic tests
  - TESTING.md documentation for developers
  - Auto-dump on failure for e2e tests

affects:
  - Phase 07-code-quality
  - Future debugging work

tech-stack:
  added:
    - DiagnosticInfo struct with comprehensive fields
    - Diagnostic capture infrastructure
    - Test diagnostic utilities
  patterns:
    - Capture-on-failure pattern for test debugging
    - Redaction of sensitive values in diagnostics
    - Human-readable diagnostic formatting

key-files:
  created:
    - pkgs/artifacts/tests/e2e/diagnostics.rs
    - pkgs/artifacts/TESTING.md
  modified:
    - pkgs/artifacts/src/cli/headless.rs
    - pkgs/artifacts/tests/e2e/mod.rs

key-decisions:
  - "Sensitive values (prompts, secrets) are redacted in diagnostics, not captured"
  - "Diagnostic dump directory is /tmp/artifacts_test_failures/ with timestamps"
  - "Format uses visual separators for human readability, not debug formatting"
  - "Both successful and failed generations capture full diagnostic info"

patterns-established:
  - "Test failure diagnostics: Always capture config, environment, and error context"
  - "Security in testing: Redact sensitive values rather than capture and filter"
  - "Human-readable output: Use sections with clear headers, not JSON or debug"
  - "Auto-dump on failure: Update existing tests to use diagnostic capture"

duration: 25 min
completed: 2026-02-16
---

# Phase 6 Plan 5: Diagnostic Tooling for Test Failure Investigation

**Diagnostic capture infrastructure with human-readable output format and
auto-dump on test failure**

## Performance

- **Duration:** 25 min
- **Started:** 2026-02-16T17:43:04Z
- **Completed:** 2026-02-16T18:08:21Z
- **Tasks:** 4
- **Files modified:** 4

## Accomplishments

- Created DiagnosticInfo struct capturing config, environment, and temp file
  contents
- Implemented generate_single_artifact_with_diagnostics() for test debugging
- Added 6 diagnostic tests covering capture, formatting, and redaction
- Created TESTING.md with comprehensive troubleshooting documentation
- Updated e2e_single_artifact_is_created to auto-dump diagnostics on failure

## Task Commits

Each task was committed atomically:

1. **Task 1: Create diagnostic capture structure** - `d2aaa43` (feat)
2. **Task 2: Create diagnostics.rs test module** - `efa65d0` (test)
3. **Task 3: Add auto-dump on test failure** - `828271f` (feat)
4. **Task 4: Integrate diagnostics and add documentation** - `bfd9016` (docs)

**Plan metadata:** `TBD` (docs: complete plan)

## Files Created/Modified

- `pkgs/artifacts/src/cli/headless.rs` - Added DiagnosticInfo struct and
  generate_single_artifact_with_diagnostics()
- `pkgs/artifacts/tests/e2e/diagnostics.rs` - New test module with 6 diagnostic
  tests
- `pkgs/artifacts/tests/e2e/mod.rs` - Updated to use diagnostic capture and
  re-export utilities
- `pkgs/artifacts/TESTING.md` - Comprehensive testing guide and troubleshooting
  documentation

## Decisions Made

- Sensitive values (prompts, secrets) are redacted in diagnostics rather than
  captured and filtered later
- Diagnostic dump directory is /tmp/artifacts_test_failures/ with
  timestamp-based filenames
- Output format uses visual separators (═══, ───) for human readability
- Both successful and failed generations capture full diagnostic info (success
  enables verification of capture)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - all tasks completed as specified.

## Next Phase Readiness

- Diagnostic tooling complete and ready for use in Phase 07-code-quality
- TESTING.md provides foundation for test documentation
- 6 new diagnostic tests passing (36 total e2e tests now passing)
- Pattern established for adding diagnostic capture to future tests

---

_Phase: 06-integration-testing_ _Completed: 2026-02-16_
