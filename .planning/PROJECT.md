# NixOS Artifacts Store - v3.0 TUI Polish

## What This Is

A system that unifies handling of artifacts (secrets and generated files) in
NixOS flakes through a common abstraction over multiple backends. The Rust CLI
uses a fully async background job architecture with tokio channels, ensuring
the TUI remains responsive during all script execution while maintaining the Elm
Architecture pattern.

## Current State

**Shipped:** v3.0 TUI Polish — 2026-02-18  
**Status:** Production-ready with polished TUI UX and comprehensive test coverage  
**Tests:** 122 passing (up from 97 at v2.0)  
**Requirements:** 20/20 v3 requirements complete

**Key Achievements:**

- **Shared artifact status** — Correct icons (needs-generation/up-to-date) instead of stuck "pending"
- **Smart generator selection** — Auto-skips dialog when only one unique generator
- **TUI error handling** — Clear stderr messages on failures, zero stdout/stderr pollution during normal operation
- **Script output visibility** — Real-time stdout/stderr display in TUI detail view
- **Enhanced generator dialog** — Rich context: artifact name, description, prompts, machines, users

**Performance:**

- Sub-50ms event polling during effect execution
- Zero blocking calls in TUI runtime loop
- Graceful shutdown with in-flight command completion
- Sequential effect processing (FIFO) prevents race conditions
- Feature-gated logging: zero overhead when disabled

---

## Current Milestone: v4.0 Regeneration Safety

**Goal:** Add a confirmation dialog before regenerating existing artifacts to prevent accidental overwrites.

**Target features:**

- Confirmation dialog when user attempts to regenerate an existing artifact
- "Leave" as default option (safe choice)
- "Regenerate" as explicit opt-in action
- Clear warning that the old artifact will be overwritten
- Status text shows "Regenerating" instead of "Generating" for existing artifacts

---

<details>
<summary>📦 v3.0 Project Evolution (click to expand)</summary>

### v3.0 Delivered Features

**Shared Artifact Status Fixes:**

- Fixed missing `SharedCheckSerializationResult` handler in update.rs
- Shared artifacts transition from "pending" to correct final status
- Status aggregation properly calculates combined status across machines
- Visual status matches actual backend state after check_serialization

**Smart Generator Selection:**

- Skips dialog when only one unique generator (compared by Nix store path)
- Shows selection dialog with full context when multiple generators exist
- Displays machine name, user name, and home-manager vs nixos source type
- Nix store path comparison for true uniqueness

**TUI Error Handling:**

- TUI initialization failures print clear error to stderr before exit
- Terminal restoration failures print error to stderr
- All runtime errors visible in TUI interface, not stdout/stderr
- Panic handler prints to stderr and attempts terminal restoration
- When `--log-file` provided, all non-error output goes to log file only

**Script Output Visibility:**

- Script stdout captured and stored for TUI display
- Script stderr captured and stored alongside stdout
- Real-time output display during script execution (streamed)
- Previous script output accessible in artifact detail view
- Output capture works for both single and shared artifacts

**Enhanced Generator Dialog:**

- Displays artifact name prominently
- Shows optional artifact description from Nix config
- Lists all prompt descriptions before generator selection
- Indicates when artifact is shared vs per-machine
- Lists all machines and users that reference the artifact
- Clean section-based layout with line separators

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

### Validated (Shipped in v3.0)

- ✓ **Fix shared artifact status icons** — Show correct status (needs-generation/up-to-date) instead of pending — v3.0
- ✓ **Smart generator selection** — Skip dialog when only one unique generator (same Nix store path) — v3.0
- ✓ **TUI error display** — Show errors when TUI fails, without stdout/stderr pollution — v3.0
- ✓ **Script output visibility** — Display stdout/stderr from scripts in TUI — v3.0
- ✓ **Enhanced generator dialog** — Show machine/user/home-manager context, shared status, artifact name, prompt descriptions — v3.0

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
- 122 total tests (comprehensive coverage)

**v3.0 Technical Context:**

All v3.0 UX issues addressed:

1. ✓ **Status display bug:** Shared artifacts now show correct calculated status
2. ✓ **Generator selection:** Auto-skips when only one unique generator
3. ✓ **Error handling:** TUI failures show user-friendly errors to stderr
4. ✓ **Script output:** Real-time stdout/stderr display in TUI detail view
5. ✓ **Context display:** Rich generator dialog with full context information

**v2.0 Achievements:**

- Complete e2e test suite verifying actual artifact creation
- Headless API for CI/integration use cases
- Code quality: 30+ refactored functions under 50 lines
- Smart logging with feature flags (zero-cost when disabled)
- 18/18 v2 requirements delivered

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
| **Option<String> for description**| Backward compatibility            | ✓ Good — optional field pattern      |
| **Description from first artifact**| Consistent with prompts/files    | ✓ Good — shared aggregation works    |

---

_Last updated: 2026-02-18 after v3.0 milestone complete_
