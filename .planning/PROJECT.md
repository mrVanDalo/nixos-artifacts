# NixOS Artifacts Store

## Current State

**Shipped:** v4.1 Code Quality & Documentation Cleanup — 2026-02-23\
**Status:** Production-ready with zero compiler warnings, comprehensive
documentation, and audited dependencies\
**Tests:** 131 passing tests (up from 122 at v4.0)\
**Requirements:** 24/24 v4.1 requirements complete (all LINT, DEAD, FILE, DOC,
DEPS)

**Key Achievements:**

- **Zero compiler warnings** — Main code and tests compile with zero rustc and
  clippy warnings
- **Zero cargo doc warnings** — Comprehensive Rust documentation with clean
  intra-doc links
- **Dead code eliminated** — All unused imports, variables removed; all
  `#[allow(dead_code)]` justified
- **File cleanup** — Removed 2 orphaned documentation files, verified all
  CLAUDE.md and README.md current
- **Dependency audit** — All 11 dependencies verified actively used
  (cargo-machete), no unused features

---

## What This Is

A system that unifies handling of artifacts (secrets and generated files) in
NixOS flakes through a common abstraction over multiple backends. The Rust CLI
uses a fully async background job architecture with tokio channels, ensuring the
TUI remains responsive during all script execution while maintaining the Elm
Architecture pattern.

---

## Core Value

**Current:** The TUI must never freeze during long-running operations — all
effect execution runs in a background job while the TUI remains interactive.

---

## Requirements

### Validated (Shipped in v4.1)

- ✓ **Zero rustc warnings** — Main code compiles with zero compiler warnings —
  v4.1
- ✓ **Zero clippy warnings** — Main code passes clippy with zero warnings — v4.1
- ✓ **Zero test rustc warnings** — Tests compile with zero compiler warnings —
  v4.1
- ✓ **Zero test clippy warnings** — Tests pass clippy with zero warnings — v4.1
- ✓ **Pedantic lints addressed** — 90% reduction with documented justifications
  — v4.1
- ✓ **No unused functions** — All functions called or justified — v4.1
- ✓ **No unused variables** — All variables used or prefixed — v4.1
- ✓ **No unused imports** — All imports referenced — v4.1
- ✓ **No unreachable code** — All paths reachable or justified — v4.1
- ✓ **Justified dead_code attributes** — All attributes have explanatory
  comments — v4.1
- ✓ **Orphaned files removed** — options.adoc and
  backend-implementation-guide.md removed — v4.1
- ✓ **No empty files** — All files contain content — v4.1
- ✓ **CLAUDE.md files current** — All 3 CLAUDE.md files verified — v4.1
- ✓ **README.md files current** — All 2 README.md files verified — v4.1
- ✓ **Module documentation** — All public modules have module-level docs — v4.1
- ✓ **Function documentation** — All public functions documented — v4.1
- ✓ **Type documentation** — All public structs/enums documented — v4.1
- ✓ **Trait documentation** — All trait implementations documented — v4.1
- ✓ **Complex logic documented** — Inline comments explain "why" — v4.1
- ✓ **Clean cargo doc** — `cargo doc` produces zero warnings — v4.1
- ✓ **Public API examples** — Usage examples in doc comments — v4.1
- ✓ **Safety sections** — Panics/Errors/Safety documented — v4.1
- ✓ **Dependencies verified** — All 11 deps actively used — v4.1
- ✓ **Features verified** — All features exercised — v4.1
- ✓ **Duplicate deps documented** — Unavoidable transitive duplicates noted —
  v4.1

### Validated (Shipped in v4.0)

- ✓ **Regeneration confirmation dialog** — "Leave" default prevents accidental
  overwrites — v4.0
- ✓ **Regeneration explicit opt-in** — "Regenerate" button for explicit
  confirmation — v4.0
- ✓ **Clear overwrite warning** — Dialog warns that old artifact will be
  overwritten — v4.0
- ✓ **Regenerating status text** — Shows "Regenerating" not "Generating" for
  existing artifacts — v4.0
- ✓ **Chronological log view** — Expandable Check/Generate/Serialize sections —
  v4.0
- ✓ **Backend developer documentation** — Comprehensive guide with lifecycle
  diagrams — v4.0
- ✓ **Model-based testing** — Elm Architecture tests with state transitions —
  v4.0

### Validated (Shipped in v3.0)

- ✓ **Fix shared artifact status icons** — Show correct status
  (needs-generation/up-to-date) instead of pending — v3.0
- ✓ **Smart generator selection** — Skip dialog when only one unique generator
  (same Nix store path) — v3.0
- ✓ **TUI error display** — Show errors when TUI fails, without stdout/stderr
  pollution — v3.0
- ✓ **Script output visibility** — Display stdout/stderr from scripts in TUI —
  v3.0
- ✓ **Enhanced generator dialog** — Show machine/user/home-manager context,
  shared status, artifact name, prompt descriptions — v3.0

### Validated (Shipped in v2.0)

- ✓ **End-to-end integration tests** — 33+ tests verifying artifact creation —
  v2.0 (Phases 5-8)
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

### Out of Scope

- WebSocket/network-based background job — Local process architecture is
  sufficient and simpler
- Concurrent effect execution in v1.0 — Sequential processing chosen for
  correctness and simplicity
- Progress bars — Not needed for atomic effects
- Progress reporting during long-running effects — No current requirement
- Cancellation of in-flight effects via user input — No current requirement
- Effect queuing with priority support — No current requirement
- Concurrent execution of independent effects — No current requirement
- Multi-threaded script execution for independent artifacts — No current
  requirement

---

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
- cargo-machete for dependency auditing
- 131 total tests (comprehensive coverage)

**v4.1 Technical Context:**

All v4.1 code quality requirements delivered:

1. ✓ **Zero rustc warnings:** `cargo build` completes with zero warnings
2. ✓ **Zero clippy warnings:** `cargo clippy` completes with zero warnings
3. ✓ **Dead code eliminated:** All unused imports/variables removed, all
   `#[allow(dead_code)]` justified
4. ✓ **File cleanup:** Removed 2 orphaned docs files, verified all
   CLAUDE.md/README.md current
5. ✓ **Documentation:** 100+ doc comments added, `cargo doc` produces zero
   warnings
6. ✓ **Dependencies:** All 11 deps verified actively used with cargo-machete

**v4.0 Achievements:**

- Regeneration confirmation dialog with "Leave" default
- Chronological log view with expandable sections
- 600+ line backend developer documentation
- 9 model-based tests for Elm Architecture

**v3.0 Achievements:**

- All v3.0 UX issues addressed:
  - Shared artifacts show correct calculated status
  - Smart generator selection auto-skips when only one unique generator
  - TUI failures show user-friendly errors to stderr
  - Real-time stdout/stderr display in TUI detail view
  - Rich generator dialog with full context information

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

---

## Constraints

- **Tech stack:** ratatui + tokio (no new runtime dependencies)
- **Compatibility:** All existing effect types supported
- **Architecture:** Elm Architecture pattern preserved
- **Isolation:** Bubblewrap sandboxing maintained
- **State:** Shared artifact aggregation working
- **Error handling:** Don't pollute stdout/stderr unless TUI fails
- **Code quality:** Zero compiler warnings maintained

---

## Key Decisions

| Decision                            | Rationale                          | Outcome                               |
| ----------------------------------- | ---------------------------------- | ------------------------------------- |
| **Unbounded channels**              | TUI must never block on send       | ✓ Good — no backpressure issues       |
| **Sequential processing**           | Avoid race conditions              | ✓ Good — FIFO order correct           |
| **spawn_blocking for I/O**          | Required for subprocess in async   | ✓ Good — scripts don't block runtime  |
| **CancellationToken**               | Clean shutdown integration         | ✓ Good — graceful exit works          |
| **Two-level timeout**               | Script-level 30s + task-level 35s  | ✓ Good — comprehensive coverage       |
| **Feature-gated logging**           | Zero cost when disabled            | ✓ Good — no runtime overhead          |
| **Fail-open check_serialization**   | Assume generation on error         | ✓ Good — safe default                 |
| **Option<String> for description**  | Backward compatibility             | ✓ Good — optional field pattern       |
| **Description from first artifact** | Consistent with prompts/files      | ✓ Good — shared aggregation works     |
| **exists flag pattern**             | Separate existence from generation | ✓ Good — precise regeneration control |
| **Leave as default**                | Prevents accidental overwrites     | ✓ Good — safety-first UX              |
| **StateCapture struct**             | Documents Elm Architecture chain   | ✓ Good — living documentation         |
| **Standalone BACKEND_GUIDE.md**     | Copy-paste ready for other repos   | ✓ Good — portable documentation       |
| **Pedantic warnings documented**    | 90% fixed, rest justified          | ✓ Good — maintainable codebase        |
| **cargo-machete for deps**          | Automated unused dep detection     | ✓ Good — verified all 11 deps used    |
| **Intra-doc links**                 | Automatic navigation in docs       | ✓ Good — clean cargo doc output       |

---

_Project: NixOS Artifacts Store_\
_Last updated: 2026-02-23 after v4.1 milestone complete_
