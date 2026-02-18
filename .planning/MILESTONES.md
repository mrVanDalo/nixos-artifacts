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

