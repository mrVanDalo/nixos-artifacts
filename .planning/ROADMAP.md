# Roadmap: NixOS Artifacts Store

**Current Version:** v3.0 ✅ SHIPPED  
**Last Updated:** 2026-02-18

---

## Milestones

- ✅ **v1.0 Background Job Refactor** — Phases 1-4 (shipped 2026-02-15) — [Archive](milestones/v1.0-ROADMAP.md)
- ✅ **v2.0 Robustness** — Phases 5-8 (shipped 2026-02-17) — [Archive](milestones/v2.0-ROADMAP.md)
- ✅ **v3.0 TUI Polish** — Phases 9-13 (shipped 2026-02-18) — [Archive](milestones/v3.0-ROADMAP.md)
- 🚧 **v4.0 Regeneration Safety** — Phase 14 (in progress)

---

## Current Status

**v3.0 TUI Polish complete.** All UX improvements shipped:
- Shared artifact status fixes
- Smart generator selection
- TUI error handling
- Script output visibility
- Enhanced generator dialog

**v4.0 Regeneration Safety** — Now in progress:
- Phase 14: Regeneration Confirmation Dialog (4 plans planned)
- All 7 v4.0 requirements mapped to Phase 14

---

## Phase Overview

| #   | Phase                         | Milestone | Status      | Completed  |
| --- | ----------------------------- | --------- | ----------- | ---------- |
| 1   | Foundation                    | v1.0      | Complete    | 2026-02-15 |
| 2   | Single Artifacts              | v1.0      | Complete    | 2026-02-15 |
| 3   | Shared Artifacts              | v1.0      | Complete    | 2026-02-15 |
| 4   | Robustness                    | v1.0      | Complete    | 2026-02-15 |
| 5   | Validation — Testing          | v2.0      | Complete    | 2026-02-16 |
| 6   | Integration Testing           | v2.0      | Complete    | 2026-02-16 |
| 7   | Code Quality                  | v2.0      | Complete    | 2026-02-17 |
| 8   | Smart Logging                 | v2.0      | Complete    | 2026-02-17 |
| 9   | Shared Artifact Status Fixes  | v3.0      | Complete    | 2026-02-18 |
| 10  | Smart Generator Selection     | v3.0      | Complete    | 2026-02-18 |
| 11  | Error Handling Improvements   | v3.0      | Complete    | 2026-02-18 |
| 12  | Script Output Visibility      | v3.0      | Complete    | 2026-02-18 |
| 13  | Enhanced Generator Dialog     | v3.0      | Complete    | 2026-02-18 |
| 14  | Regeneration Confirmation     | v4.0      | Not started | -          |

**Total:** 13 phases complete, 1 phase planned

---

## Completed Milestones

<details>
<summary>✅ v3.0 TUI Polish (Phases 9-13) — SHIPPED 2026-02-18</summary>

**Goal:** Fix bugs and improve UX in the TUI

**Requirements:**
- UI-01 to UI-05: UI/UX fixes
- STAT-01, STAT-02: Status display
- OUT-01 to OUT-04: Output capture
- ERR-01 to ERR-04: Error handling
- GEN-01 to GEN-04: Generator selection
- DIALOG-01 to DIALOG-05: Enhanced dialog

**Phases:**

- [x] Phase 9: Shared Artifact Status Fixes (4/4 plans) — 2026-02-18
- [x] Phase 10: Smart Generator Selection (2/2 plans) — 2026-02-18
- [x] Phase 11: Error Handling Improvements (3/3 plans) — 2026-02-18
- [x] Phase 12: Script Output Visibility (4/4 plans) — 2026-02-18
- [x] Phase 13: Enhanced Generator Dialog (2/2 plans) — 2026-02-18

**Key Accomplishments:**
- Shared artifacts show correct status icons
- Smart generator selection (skips dialog when one unique generator)
- TUI errors display to stderr without polluting stdout
- Script output visible in real-time during execution
- Enhanced generator dialog with rich context

**Archive:** [v3.0-ROADMAP.md](milestones/v3.0-ROADMAP.md)

</details>

<details>
<summary>✅ v2.0 Robustness (Phases 5-8) — SHIPPED 2026-02-17</summary>

**Goal:** End-to-end tests, code quality, smart logging

**Requirements:**
- TEST-01 to TEST-06: End-to-end verification
- QUAL-01 to QUAL-07: Code readability & structure
- LOG-01 to LOG-06: Opt-in debug logging

**Phases:**

- [x] Phase 5: Validation — Testing (3/3 plans) — 2026-02-16
- [x] Phase 6: Integration Testing (5/5 plans) — 2026-02-16
- [x] Phase 7: Code Quality (3/3 plans) — 2026-02-17
- [x] Phase 8: Smart Logging (3/3 plans) — 2026-02-17

**Key Accomplishments:**
- 33+ e2e tests across 5 test modules
- Headless API for programmatic artifact generation
- Diagnostic tooling with auto-dump on failure
- 12 refactored handler functions (all under 50 lines)
- 18 helper functions in serialization.rs
- Feature-gated logging with `--log-file` and `--log-level` CLI arguments
- Zero-cost logging (no overhead when disabled)

**Audit Report:** [v2.0-MILESTONE-AUDIT.md](milestones/v2.0-MILESTONE-AUDIT.md)

</details>

<details>
<summary>✅ v1.0 Background Job Refactor (Phases 1-4) — SHIPPED 2026-02-15</summary>

See [v1.0-ROADMAP.md](milestones/v1.0-ROADMAP.md)

</details>

### 🚧 v4.0 Regeneration Safety (In Progress)

**Milestone Goal:** Add a confirmation dialog before regenerating existing artifacts to prevent accidental overwrites.

**Requirements:**
- REGEN-01 to REGEN-07: Regeneration confirmation and safety

#### Phase 14: Regeneration Confirmation Dialog

**Goal:** Users must explicitly confirm before overwriting existing artifacts, with clear warnings and safe defaults

**Depends on:** Phase 13

**Requirements:** REGEN-01, REGEN-02, REGEN-03, REGEN-04, REGEN-05, REGEN-06, REGEN-07

**Success Criteria** (what must be TRUE):

1. User sees confirmation dialog when attempting to regenerate an artifact that already exists (REGEN-01)
2. Dialog default selection is "Leave" (safe choice, prevents accidental overwrite) (REGEN-02)
3. User can explicitly choose "Regenerate" to proceed with overwrite (REGEN-03)
4. Dialog clearly warns that the old artifact will be overwritten (REGEN-04)
5. Status text shows "Regenerating" instead of "Generating" during overwrite operations (REGEN-05)
6. Confirmation dialog appears for both single artifacts and shared artifacts (REGEN-06)
7. Dialog supports keyboard navigation (arrow keys to select, Enter to confirm, Esc to cancel) (REGEN-07)

**Plans:** 4 plans

Plans:

- [ ] 14-01: Detect existing artifact state and trigger confirmation dialog — Add exists flag to artifact entries, extend check_serialization result, wire detection through model builder
- [ ] 14-02: Implement dialog UI with Leave/Regenerate options and warning text — Create ConfirmRegenerate screen state, dialog view with side-by-side buttons, keyboard navigation
- [ ] 14-03: Update status text to show "Regenerating" for existing artifacts — Add exists to GeneratingState, update generating/list views with full sentence format
- [ ] 14-04: Add comprehensive tests for confirmation dialog behavior — State transition tests, visual snapshots, keyboard navigation tests, status text tests

### Phase 15: Chronological Log View with Expandable Sections

**Goal:** [To be planned]
**Depends on:** Phase 14
**Plans:** 0 plans

Plans:
- [ ] TBD (run /gsd:plan-phase 15 to break down)

### Phase 16: Backend Developer Documentation for Custom Serializations

**Goal:** Create comprehensive backend developer documentation in Antora format PLUS a standalone BACKEND_GUIDE.md file that can be copied to other repositories so AI assistants have enough context to write backends

**Depends on:** Phase 15
**Plans:** 0 plans

Plans:
- [ ] TBD (run /gsd:plan-phase 16 to break down)

### Phase 17: Model-based testing with full state capture

**Goal:** [To be planned]
**Depends on:** Phase 16
**Plans:** 0 plans

Plans:
- [ ] TBD (run /gsd:plan-phase 17 to break down)

---

_Updated: 2026-02-18 — v3.0 TUI Polish complete_
