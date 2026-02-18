# State: v3.0 TUI Polish ○ ROADMAP READY

**Project:** NixOS Artifacts Store — v3.0 TUI Polish
**Current Milestone:** v3.0 ○ ROADMAP READY
**Status:** Phase 12 complete
**Last Updated:** 2026-02-18

---

## Project Reference

See: [.planning/PROJECT.md](./PROJECT.md) (updated 2026-02-18)

**Core Value:** The TUI must never freeze during long-running operations — all
effect execution runs in a background job while the TUI remains interactive.

**Current Focus:** Phase 12 - Script output visibility (4 plans)

---

## Current Position

| Aspect       | Status                       |
| ------------ | ---------------------------- |
| Milestone    | v3.0 ○ ROADMAP READY         |
| Phase        | 12-script-output-visibility  |
| Plan         | 04-complete                  |
| Requirements | 20 v1 requirements mapped    |
| Tests        | 118 passing                  |
| Previous     | Plan 12-04 complete          |

### Progress Bar

```
[████████████████████████░░░░░░░░] 92% complete — Phase 12 complete (4 of 4 plans complete)
```

---

## Accumulated Context

### Decisions from v1.0 and v2.0

All decisions preserved in PROJECT.md Validated section.

### Key Decisions from v2.0

- **Test approach:** Programmatic headless tests that invoke CLI and verify backend storage
- **Logging strategy:** Opt-in via `--log-file <path>` argument with feature flags
- **Refactoring goal:** Flatten call chains, eliminate abbreviations, improve readability
- **State machine testing:** Dual assertion strategy - verify both command variants AND final model state
- **Feature-gated logging:** Zero-cost when disabled via Cargo features

### Technical Debt

**From v1.0 (all addressed):**

- ✅ End-to-end tests verify actual artifact creation in backend storage
- ✅ Functions have flattened call chains
- ✅ No abbreviated variable names
- ✅ No hardcoded debug logging paths

**v2.0 Technical Debt (cosmetic):**

- 3 orchestration functions in serialization.rs slightly exceed 50-line limit
- Acceptable: delegate to 15+ well-named helpers, code is readable and maintainable

### Completed v2.0

**Phase 5: Validation — Testing:**
- State machine simulation tests with dual assertion strategy
- 15 async tests covering full lifecycle transitions
- 80%+ coverage for async channel components

**Phase 6: Integration Testing:**
- 33+ e2e tests across 5 test modules
- Headless API for programmatic artifact generation
- Backend storage verification (TEST-03, TEST-04)
- Shared artifact tests (TEST-05)
- Diagnostic tooling with auto-dump on failure (TEST-06)

**Phase 7: Code Quality:**
- 12 refactored handler functions (all under 50 lines)
- 18 helper functions in serialization.rs
- Flattened call chains (no f(g(h(x))))
- Descriptive variable names (no abbreviations)

**Phase 8: Smart Logging:**
- `--log-file` and `--log-level` CLI arguments
- Feature-gated with zero-cost when disabled
- Macro API: error!, warn!, info!, debug!
- Hardcoded `/tmp/artifacts_debug.log` completely removed
- 11 comprehensive logging tests

---

## TODOs

- [x] v2.0 milestone complete
- [x] Archive v2.0 (milestones/v2.0-*)
- [x] Git tag v2.0 created
- [x] Define v3.0 goals
- [x] Complete v3.0 planning (requirements, roadmap) — 20 requirements → 5 phases
- [x] Phase 9: Shared artifact status fix (UI-01, STAT-01, STAT-02)
  - [x] Plan 09-01: Status tracking infrastructure
  - [x] Plan 09-02: Error state handling
  - [x] Plan 09-03: Status display polish
  - [x] Plan 09-04: Gap closure verification ✓
- [x] Phase 10: Smart generator selection (UI-02, GEN-01-04)
  - [x] Plan 10-01: Smart generator selection logic
  - [x] Plan 10-02: Enhanced dialog context
- [x] Phase 11: TUI error display (UI-03, ERR-01-04)
  - [x] Plan 11-01: Pre-terminal error handling
  - [x] Plan 11-02: Enhanced panic handler and terminal restoration
  - [x] Plan 11-03: TUI error display audit
  - [x] Phase 12: Script output visibility (UI-04, OUT-01-04) ✓
    - [x] Plan 12-01: Data flow pipeline for script output ✓
    - [x] Plan 12-02: StepLogs helper methods ✓
    - [x] Plan 12-03: Script output display in TUI views ✓
    - [x] Plan 12-04: Real-time streaming output ✓
  - [ ] Phase 13: Enhanced generator dialog (UI-05, DIALOG-01-05)
- [ ] Phase 13: Enhanced generator dialog (UI-05, DIALOG-01-05)

---

## Session Continuity

### Last Session

**Date:** 2026-02-18
**Activity:** Completed Plan 12-01: Script Output Visibility Data Flow Pipeline
**Summary:**
- Added ScriptOutput struct with stdout_lines and stderr_lines fields
- Updated all EffectResult variants to use ScriptOutput instead of Option<String>
- Implemented complete result_to_message conversion in runtime.rs
- Added helper methods: from_captured(), from_message(), default()
- Updated background.rs to convert CapturedOutput to ScriptOutput
- All 118 tests pass

**Decisions Made:**
- ScriptOutput::from_captured() converts from CapturedOutput (lines with stream markers)
- ScriptOutput::from_message() creates ScriptOutput with message for error cases
- EffectResult variants now carry ScriptOutput preserving stdout/stderr separation

### Current Session

**Date:** 2026-02-18
**Activity:** Completed Plan 12-04: Real-time streaming output infrastructure
**Summary:**
- Added OutputStream enum and EffectResult::OutputLine variant to channels.rs
- Added Msg::OutputLine variant to message.rs with OutputStream from model
- Implemented handle_output_line() in update.rs that appends to current step logs
- Added streaming infrastructure in background.rs with result_tx channel
- Mapped stdout to LogLevel::Output ("|") and stderr to LogLevel::Error ("!")
- All 35 TUI tests pass
- Requirement OUT-03 (real-time output updates) satisfied

**Decisions Made:**
- Used separate OutputStream enums in channels and model with From conversion
- Streaming output appends to currently selected_log_step
- result_tx channel enables background task to send incremental updates

---

## Quick Links

- [PROJECT.md](./PROJECT.md) — Core value and requirements
- [REQUIREMENTS.md](./REQUIREMENTS.md) — v3.0 requirements (creating)
- [ROADMAP.md](./ROADMAP.md) — Phase structure (creating)
- [Milestones](./milestones/) — Archived milestones
- [12-01-SUMMARY.md](./phases/12-script-output-visibility/12-01-SUMMARY.md) — Plan 12-01 completion
