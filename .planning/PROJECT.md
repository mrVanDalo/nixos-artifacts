# NixOS Artifacts Store - v3.0 TUI Polish

## What This Is

A system that unifies handling of artifacts (secrets and generated files) in
NixOS flakes through a common abstraction over multiple backends. The Rust CLI
uses a fully async background job architecture with tokio channels, ensuring
the TUI remains responsive during all script execution while maintaining the Elm
Architecture pattern.

## Current State

**Shipped:** v2.0 Robustness — 2026-02-17\
**Status:** Production-ready with comprehensive test coverage and smart logging\
**Tests:** 97 passing (33 e2e tests + 64 existing)\
**Coverage:** 18/18 v2 requirements complete (29/32 must-haves)

**Key Achievements:**

- **TUI never freezes** during long-running operations
- **End-to-end tests** verify artifacts actually get created in backend storage
- **Smart logging** with `--log-file` CLI argument (opt-in, zero-cost)
- **Code quality** improvements: 30+ helper functions, flattened call chains, no abbreviations
- **Headless API** for programmatic artifact generation without TUI

**Performance:**

- Sub-50ms event polling during effect execution
- Zero blocking calls in TUI runtime loop
- Graceful shutdown with in-flight command completion
- Sequential effect processing (FIFO) prevents race conditions
- Feature-gated logging: zero overhead when disabled

---

## Current Milestone: v3.0 TUI Polish

**Goal:** Fix bugs and improve UX in the TUI for better visibility and smarter interactions

**Target Features:**

1. **Fix shared artifact status icons** — Correct status display (needs-generation/up-to-date instead of pending)
2. **Smart generator selection** — Skip dialog when only one unique generator exists (same Nix store path)
3. **TUI error display** — Show errors when TUI fails, without polluting stdout/stderr otherwise
4. **Script output visibility** — Display stdout/stderr from check/generator/serialize scripts in TUI
5. **Enhanced generator dialog** — Show machine/user/home-manager context, shared status, artifact name, prompt descriptions

---

<details>
<summary>📦 v3.0 Delivered Features</summary>

*Not yet started — placeholder for completed work*

</details>

<details>
<summary>📦 v2.0 Project Evolution (click to expand)</summary>

### v2.0 Delivered Features

**Testing & Reliability:**

- 33+ end-to-end tests across 5 test modules
- Headless API for programmatic artifact generation
- Backend storage verification (artifacts actually exist after generation)
- Diagnostic tooling with auto-dump on failure
- Shared artifact test coverage

**Code Quality:**

- 12 refactored handler functions (all under 50 lines)
- 18 helper functions extracted from serialization.rs
- Descriptive variable names (no `cfg`, `hdl`, `ctx`)
- Flattened call chains (f(g(h(x))) → extract and chain)
- Single-responsibility functions with success/failure split

**Smart Logging:**

- `--log-file <path>` CLI argument
- `--log-level <error|warn|info|debug>` filtering
- Feature-gated: zero cost when disabled
- Macro API: `error!`, `warn!`, `info!`, `debug!`
- Real-time streaming with flush after each entry
- Hardcoded `/tmp/artifacts_debug.log` completely removed

**Technical Debt Note:**

- 3 orchestration functions in serialization.rs slightly exceed 50-line limit
- These delegate to 15+ well-named helpers — readable and maintainable
- Considered acceptable technical debt (cosmetic, not functional)

</details>

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

### Validated (Shipped in v2.0)

- ✓ **End-to-end integration tests** — 33+ tests verifying artifact creation — v2.0 (Phases 5-8)
- ✓ **Headless API** — Programmatic generation without TUI — v2.0
- ✓ **Backend verification** — Tests confirm artifacts exist in storage — v2.0
- ✓ **Code quality** — Flattened call chains, no abbreviations — v2.0
- ✓ **Smart logging** — `--log-file` opt-in with feature flags — v2.0
- ✓ **Zero-cost logging** — No overhead when disabled — v2.0
- ✓ **Diagnostic tooling** — Auto-dump on test failure — v2.0

### Validated (Shipped in v1.0)

- ✓ **Background job architecture** — tokio channels for foreground/background
  communication — v1.0 (Phases 1-4)
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

### Active (v3.0 TUI Polish)

- [ ] **Fix shared artifact status icons** — Show correct status (needs-generation/up-to-date) instead of pending — v3.0
- [ ] **Smart generator selection** — Skip dialog when only one unique generator (same Nix store path) — v3.0
- [ ] **TUI error display** — Show errors when TUI fails, without stdout/stderr pollution — v3.0
- [ ] **Script output visibility** — Display stdout/stderr from scripts in TUI — v3.0
- [ ] **Enhanced generator dialog** — Show machine/user/home-manager context, shared status, artifact name, prompt descriptions — v3.0

### Future Ideas

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
- Progress bars — Not needed for atomic effects

## Context

**Current Stack:**

- Rust CLI with tokio async runtime
- ratatui for TUI with Elm Architecture
- tokio mpsc unbounded channels for foreground/background
- spawn_blocking for all subprocess I/O
- Bubblewrap for script isolation
- tempfile crate for temp directory management
- insta + insta_cmd for snapshot testing
- serial_test for test isolation
- 97 total tests (33 e2e + 64 unit/integration)

**v3.0 Technical Context:**

The TUI has several UX issues that need addressing:

1. **Status display bug:** Shared artifacts show "pending" status instead of calculated status
2. **Generator selection:** Always prompts even when only one unique generator exists
3. **Error handling:** TUI failures don't show user-friendly errors
4. **Script output:** Users can't see script output during generation
5. **Context display:** Generator dialog lacks important context information

**v2.0 Achievements:**

- Complete e2e test suite verifying actual artifact creation
- Headless API for CI/integration use cases
- Code quality: 30+ refactored functions under 50 lines
- Smart logging with feature flags (zero-cost when disabled)
- 18/18 v2 requirements delivered (29/32 must-haves)

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
- **Error handling:** Don't pollute stdout/stderr unless TUI fails

## Key Decisions

| Decision                          | Rationale                         | Outcome                              |
| --------------------------------- | --------------------------------- | ------------------------------------ |
| **Unbounded channels**            | TUI must never block on send      | ✓ Good — no backpressure issues      |
| **Sequential processing**         | Avoid race conditions             | ✓ Good — FIFO order correct          |
| **spawn_blocking for I/O**        | Required for subprocess in async  | ✓ Good — scripts don't block runtime |
| **CancellationToken**               | Clean shutdown integration        | ✓ Good — graceful exit works         |
| **Two-level timeout**               | Script-level 30s + task-level 35s | ✓ Good — comprehensive coverage      |
| **Feature-gated logging**         | Zero cost when disabled           | ✓ Good — no runtime overhead         |
| **Fail-open check_serialization** | Assume generation on error        | ✓ Good — safe default                |

---

_Last updated: 2026-02-17 after v3.0 milestone started_
