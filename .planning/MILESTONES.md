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
