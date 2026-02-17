# NixOS Artifacts Store - Background Job Refactor

## What This Is

A system that unifies handling of artifacts (secrets and generated files) in
NixOS flakes through a common abstraction over multiple backends. The Rust CLI
now uses a fully async background job architecture with tokio channels, ensuring
the TUI remains responsive during all script execution while maintaining the Elm
Architecture pattern.

## Current State

**Shipped:** v1.0 Background Job Refactor — 2026-02-15\
**Status:** Production-ready TUI with async effect handling\
**Tests:** 64 passing (21 new async tests)\
**Coverage:** 35/35 v1 requirements complete

**Key Achievement:** TUI never freezes during long-running operations. All
effect execution runs in a background job while the TUI remains interactive and
responsive.

**Performance:**

- Sub-50ms event polling during effect execution
- Zero blocking calls in TUI runtime loop
- Graceful shutdown with in-flight command completion
- Sequential effect processing (FIFO) prevents race conditions

---

<details>
<summary>📦 v1.0 Project Evolution (click to expand)</summary>

### Original Project Description (v1.0)

A system that unifies handling of artifacts (secrets and generated files) in
NixOS flakes through a common abstraction over multiple backends. This project
focused on refactoring the Rust CLI's effect handling to use a background job
pattern with tokio, ensuring the TUI remains responsive during script execution.

### Original Core Value

The TUI must never freeze during long-running operations — all effect execution
(check_serialization, generator, serialize scripts) must run in a background job
while the TUI remains interactive.

### Original Requirements (Pre-v1.0)

**Validated:**

- ✓ Artifact definitions via NixOS modules (artifacts.store.*)
- ✓ Backend plugin system via backend.toml
- ✓ TUI with Elm Architecture (Model-Update-View-Effect)
- ✓ Effect system describing side effects
- ✓ Script execution with bubblewrap isolation
- ✓ Shared vs per-machine artifact support
- ✓ Temporary file handling with tempfile crate

**Active (moved to Validated in v1.0):**

- Background job architecture with tokio channels
- TUI remains responsive during all effect operations
- Sequential processing of effects in background (FIFO queue)
- Async message passing between TUI and background job
- Replace current effect_handler.rs with new channel-based approach
- Proper shutdown handling for background job
- Error propagation from background to foreground
- State synchronization between TUI and background operations

</details>

---

## Core Value

**Current:** The TUI must never freeze during long-running operations — all
effect execution runs in a background job while the TUI remains interactive.

## Requirements

### Validated (Shipped in v1.0)

- ✓ **Background job architecture** — tokio channels for foreground/background
  communication — v1.0 (Phases 1-3)
- ✓ **Responsive TUI** — Zero blocking during effect operations — v1.0
- ✓ **Sequential processing** — FIFO queue for effects — v1.0
- ✓ **Async message passing** — EffectCommand/EffectResult via mpsc — v1.0
- ✓ **effect_handler.rs replaced** — Fully replaced with channel-based — v1.0
- ✓ **Graceful shutdown** — CancellationToken + in-flight completion — v1.0
- ✓ **Error propagation** — Full error details in TUI — v1.0
- ✓ **State synchronization** — HashMap status tracking with animation — v1.0
- ✓ **Artifact definitions** via NixOS modules — pre-existing
- ✓ **Backend plugin system** via backend.toml — pre-existing
- ✓ **Elm Architecture** (Model-Update-View-Effect) — pre-existing
- ✓ **Bubblewrap isolation** for scripts — pre-existing

### Active (v2.0 Ideas)

- [ ] Progress reporting during long-running effects
- [ ] Cancellation of in-flight effects via user input
- [ ] Effect queuing with priority support
- [ ] Concurrent execution of independent effects
- [ ] Multi-threaded script execution for independent artifacts

### Out of Scope

- WebSocket/network-based background job — Local process architecture is
  sufficient and simpler
- Concurrent effect execution in v1.0 — Sequential processing chosen for
  correctness and simplicity
- Progress bars — Not needed for atomic effects (v1 scope)

## Context

**Current Stack:**

- Rust CLI with tokio async runtime
- ratatui for TUI with Elm Architecture
- tokio mpsc unbounded channels for foreground/background
- spawn_blocking for all subprocess I/O
- Bubblewrap for script isolation
- tempfile crate for temp directory management
- 21 async tests + 43 existing tests = 64 total

**v1.0 Achievements:**

- Complete async channel architecture
- All effect types (single + shared) working via background job
- Graceful shutdown with CancellationToken
- Error display integration with full context
- File-based logging to prevent terminal corruption
- 35/35 requirements delivered

## Constraints

- **Tech stack:** ratatui + tokio (no new runtime dependencies)
- **Compatibility:** All existing effect types supported
- **Architecture:** Elm Architecture pattern preserved
- **Isolation:** Bubblewrap sandboxing maintained
- **State:** Shared artifact aggregation working

## Key Decisions

| Decision                          | Rationale                         | Outcome                              |
| --------------------------------- | --------------------------------- | ------------------------------------ |
| **Unbounded channels**            | TUI must never block on send      | ✓ Good — no backpressure issues      |
| **Sequential processing**         | Avoid race conditions             | ✓ Good — FIFO order correct          |
| **spawn_blocking for I/O**        | Required for subprocess in async  | ✓ Good — scripts don't block runtime |
| **CancellationToken**             | Clean shutdown integration        | ✓ Good — graceful exit works         |
| **Two-level timeout**             | Script-level 30s + task-level 35s | ✓ Good — comprehensive coverage      |
| **File-based logging**            | Prevent console corruption        | ✓ Good — debug output visible        |
| **Fail-open check_serialization** | Assume generation on error        | ✓ Good — safe default                |

## Current Milestone: v2.0 Robustness

**Goal:** Fix critical gaps from v1.0 — ensure artifacts actually get created
and improve code quality for long-term maintainability.

**Target features:**

1. **End-to-End Integration Tests** — Verify artifacts are actually created and
   stored correctly
2. **Code Quality Refactoring** — Shorter functions, flattened call chains, no
   abbreviations
3. **Smart Debug Logging** — Optional `--log-output <file>` argument for
   comprehensive debug logging

**Success Criteria:**

- Tests verify secrets exist at expected backend locations after generation
- Functions terminate and return results rather than deep call chains
- All variables and functions have clear, non-abbreviated names
- Debug logging is opt-in via CLI argument

---

_Last updated: 2026-02-15 after v1.0 milestone completion_
