# State: v3.0 TUI Polish ○ ROADMAP READY

**Project:** NixOS Artifacts Store — v3.0 TUI Polish
**Current Milestone:** v3.0 ○ ROADMAP READY
**Status:** Roadmap created, awaiting approval
**Last Updated:** 2026-02-18

---

## Project Reference

See: [.planning/PROJECT.md](./PROJECT.md) (updated 2026-02-18)

**Core Value:** The TUI must never freeze during long-running operations — all
effect execution runs in a background job while the TUI remains interactive.

**Current Focus:** v3.0 Roadmap — 5 phases, 20 requirements mapped

---

## Current Position

| Aspect       | Status                       |
| ------------ | ---------------------------- |
| Milestone    | v3.0 ○ ROADMAP READY         |
| Phase        | — (awaiting approval)        |
| Plan         | — (start with Phase 9)       |
| Requirements | 20 v1 requirements mapped    |
| Tests        | 97 passing (baseline)        |
| Previous     | v2.0 ✅ SHIPPED (2026-02-17) |

### Progress Bar

```
[░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 0% complete — Ready to start Phase 9
```

---

## Accumulated Context

### Decisions from v1.0 and v2.0

All decisions preserved in PROJECT.md Validated section.

### Key Decisions from v2.0

- **Test approach:** Programmatic headless tests that invoke CLI and verify
  backend storage
- **Logging strategy:** Opt-in via `--log-file <path>` argument with feature flags
- **Refactoring goal:** Flatten call chains, eliminate abbreviations, improve
  readability
- **State machine testing:** Dual assertion strategy - verify both command
  variants AND final model state
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
- [ ] Phase 9: Shared artifact status fix (UI-01, STAT-01, STAT-02)
- [ ] Phase 10: Smart generator selection (UI-02, GEN-01-04)
- [ ] Phase 11: TUI error display (UI-03, ERR-01-04)
- [ ] Phase 12: Script output visibility (UI-04, OUT-01-04)
- [ ] Phase 13: Enhanced generator dialog (UI-05, DIALOG-01-05)

---

## Session Continuity

### Last Session

**Date:** 2026-02-17
**Activity:** Completed v2.0 milestone
**Summary:** Archived v2.0 Robustness milestone with 4 phases (5-8), 28 plans, 18 requirements. Created milestone archives, updated PROJECT.md and ROADMAP.md, created git tag v2.0.

### Current Session

**Date:** 2026-02-18
**Activity:** Starting v3.0 TUI Polish milestone
**Summary:** Defined 5 TUI polish improvements: shared status icons, smart generator selection, error display, script output visibility, enhanced generator dialog.

---

## Quick Links

- [PROJECT.md](./PROJECT.md) — Core value and requirements
- [REQUIREMENTS.md](./REQUIREMENTS.md) — v3.0 requirements (creating)
- [ROADMAP.md](./ROADMAP.md) — Phase structure (creating)
- [Milestones](./milestones/) — Archived milestones
