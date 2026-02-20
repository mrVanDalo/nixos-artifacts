# Milestones

## v2.0 Robustness (Shipped: 2026-02-17)

**Phases completed:** 8 phases (5-8), 14 plans, 18 requirements  
**Test coverage:** 33+ e2e tests, 97 total tests passing  
**Audit status:** 29/32 must-haves verified (91%) — gaps_found but ready to ship  
**Git tag:** v2.0

**Key accomplishments:**

1. **End-to-End Integration Tests** — 33+ tests verifying artifacts are actually
created in backend storage. Headless API enables programmatic generation without TUI.

2. **Code Quality Refactoring** — 30+ functions refactored: 12 handlers under 50
lines, 18 helpers extracted from serialization.rs. Flattened call chains, no
abbreviations.

3. **Smart Logging** — Feature-gated `--log-file` and `--log-level` CLI arguments.
Zero-cost when disabled. Complete macro API with 11 comprehensive tests.

4. **Diagnostic Tooling** — Auto-dump on test failure with full context (config,
env vars, temp files). Makes debugging test failures straightforward.

5. **Shared Artifact Coverage** — Comprehensive tests for shared artifacts across
machines with multi-machine scenarios.

**Technical Debt:** 3 orchestration functions exceed 50-line limit (cosmetic),
all delegate to well-named helpers.

**Archive:** [v2.0-ROADMAP.md](milestones/v2.0-ROADMAP.md)  
**Audit:** [v2.0-MILESTONE-AUDIT.md](milestones/v2.0-MILESTONE-AUDIT.md)

---

## v1.0 Background Job Refactor (Shipped: 2026-02-15)

See [v1.0-ROADMAP.md](milestones/v1.0-ROADMAP.md)

---

_Updated: 2026-02-17_

## v3.0 TUI Polish (Shipped: 2026-02-18)

**Phases completed:** 5 phases (9-13), 15 plans, 5 requirements  
**Git tag:** v3.0

**Key accomplishments:**

1. **Shared Artifact Status Fixes** — Shared artifacts now display correct status
icons (needs-generation/up-to-date) instead of stuck "pending". Fixed missing
`SharedCheckSerializationResult` handler in update.rs.

2. **Smart Generator Selection** — Generator selection dialog automatically skips
when only one unique generator exists (compared by Nix store path). Shows rich
context including machine names, user names, and home-manager vs nixos sources.

3. **TUI Error Handling** — TUI initialization failures print clear errors to
stderr before exit. All runtime errors visible in TUI interface, not stdout/stderr.
Panic handler catches unwinding panics and attempts terminal restoration.

4. **Script Output Visibility** — Script stdout/stderr from check/generator/serialize
operations captured and displayed in TUI. Real-time streaming output during script
execution. Previous output accessible in artifact detail view.

5. **Enhanced Generator Dialog** — Rich dialog displays artifact name, optional
description, all prompt descriptions, shared status, and target machines/users.
Full context before generator selection.

**Archive:** [v3.0-ROADMAP.md](milestones/v3.0-ROADMAP.md)

---


## v4.0 Regeneration Safety (Shipped: 2026-02-22)

**Phases completed:** 4 phases (14-17), 13 plans, 14 tasks
**Test coverage:** 131 passing tests (up from 122 in v3.0)
**Git tag:** v4.0
**Git range:** 0bc5ff1..4a1321d (66 commits)

**Key accomplishments:**

1. **Regeneration Confirmation Dialog** — "Leave" as default prevents accidental overwrites.
   "Regenerate" as explicit opt-in. Keyboard navigation with clear warning text.
   Works for both single and shared artifacts.

2. **Chronological Log View** — Expandable/collapsible sections for Check, Generate, Serialize
   steps with Space/Enter toggle. Keyboard navigation with j/k between sections.
   Summary display when collapsed.

3. **Backend Developer Documentation** — 600+ line comprehensive guide in Antora format
   with lifecycle diagrams and quickstart templates. Standalone BACKEND_GUIDE.md
   for copy-paste to other repositories.

4. **Model-based Testing** — Elm Architecture pattern demonstrated with 9 state transition
   tests. Inputs -> Model transformations (via update) -> view rendering. Dual assertion
   pattern captures both Model state and rendered view.

**Technical Debt:** None — all v4.0 requirements delivered.

**Archive:** [v4.0-ROADMAP.md](milestones/v4.0-ROADMAP.md)  
**Requirements:** [v4.0-REQUIREMENTS.md](milestones/v4.0-REQUIREMENTS.md)

---

