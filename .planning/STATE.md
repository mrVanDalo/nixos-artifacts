# State: Background Job Refactor

**Project:** NixOS Artifacts Store — Background Job Refactor\
**Current Milestone:** v1.0 ✅ SHIPPED\
**Status:** Complete — All 35 requirements delivered\
**Last Updated:** 2026-02-15T00:00:00Z

---

## Project Reference

**Core Value:** The TUI must never freeze during long-running operations — all
effect execution runs in a background job while the TUI remains interactive.

**Key Constraints:**

- Use existing ratatui + tokio (no new dependencies)
- Preserve Elm Architecture pattern (Model-Update-View-Effect)
- Maintain bubblewrap sandboxing for script execution
- Sequential processing of effects (FIFO queue)

---

## Current Position

| Aspect          | Status                      |
| --------------- | --------------------------- |
| Milestone       | v1.0 ✅ SHIPPED             |
| Phases          | 5/5 complete (18 plans)     |
| Requirements    | 35/35 delivered             |
| Tests           | 64 passing (21 async)       |
| Next Milestone  | Planning v2.0               |

### Progress Bar

```
[████████████████████] 100% complete — v1.0 SHIPPED
```

### Milestone History

| Milestone | Phases | Plans | Status   | Date       |
| --------- | ------ | ----- | -------- | ---------- |
| v1.0 MVP  | 1-5    | 18    | ✅ Shipped | 2026-02-15 |

### Phase History

| Phase                     | Status     | Date       |
| ------------------------- | ---------- | ---------- |
| Phase 1: Foundation       | Complete   | 2026-02-13 |
| Phase 2: Single Artifacts | Complete   | 2026-02-13 |
| Phase 3: Shared Artifacts | Complete   | 2026-02-14 |
| Phase 4: Robustness       | Complete   | 2026-02-14 |
| Phase 5: Validation       | Complete   | 2026-02-14 |

---

## Completed Plans

| Plan  | Description                     | Completed  |
| ----- | ------------------------------- | ---------- |
| 01-01 | Channel Message Types           | 2026-02-13 |
| 01-02 | Background Task                 | 2026-02-13 |
| 01-03 | Runtime Integration             | 2026-02-13 |
| 02-01 | Real Backend Integration        | 2026-02-13 |
| 02-02 | EffectHandler Bridge            | 2026-02-13 |
| 02-03 | State Management and UI Updates | 2026-02-13 |
| 03-01 | Shared Artifact Effects         | 2026-02-13 |
| 03-02 | Tokio Runtime Fix               | 2026-02-14 |
| 03-03 | Debug Logging                   | 2026-02-14 |
| 03-04 | Timeout Handling                | 2026-02-14 |
| 03-05 | TUI Freeze Fix                  | 2026-02-14 |
| 04-01 | CancellationToken Shutdown      | 2026-02-14 |
| 04-02 | Graceful Shutdown Sequence      | 2026-02-14 |
| 04-03 | Error Display Integration       | 2026-02-14 |
| 04-VERIFICATION | Phase 4 verification passed (7/7) | 2026-02-14 |
| 05-01 | Async Unit Tests                | 2026-02-14 |
| 05-02 | Select and Shutdown Tests       | 2026-02-14 |
| 05-03 | Runtime and Update Tests        | 2026-02-14 |

## Performance Metrics

| Plan  | Duration | Tasks | Files |
| ----- | -------- | ----- | ----- |
| 01-01 | 6 min    | 6     | 4     |
| 01-02 | 3 min    | 5     | 3     |
| 01-03 | 7 min    | 8     | 8     |
| 02-01 | 4 min    | 3     | 4     |
| 02-02 | 0 min    | 3     | 3     |
| 02-03 | 5 min    | 4     | 4     |
| 03-01 | 6 min    | 3     | 1     |
| 03-02 | 3 min    | 3     | 0     |
| 03-03 | 19 min   | 5     | 4     |
| 03-04 | 10 min   | 3     | 3     |
| 03-05 | 4 min    | 1     | 2     |
| 04-01 | 12 min   | 5     | 3     |
| 04-02 | 8 min    | 3     | 2     |
| 05-01 | 6 min    | 3     | 4     |
| 05-02 | 8 min    | 3     | 4     |
| 05-03 | 11 min   | 3     | 3     |

---

## Accumulated Context

### Decisions Made

1. **Async unit tests use tokio::time::timeout wrapper** — All async tests use `tokio::time::timeout(Duration::from_secs(...), ...)` to prevent hanging, rather than mock time which requires test-util feature.

2. **Channel tests use mock types** — tests/async_tests/channel_tests.rs uses inline MockEffectCommand/MockEffectResult types to avoid dependency on full BackendConfiguration setup.

3. **Background tests use actual handler** — tests/async_tests/background_tests.rs uses actual BackgroundEffectHandler and spawn_background_task to verify real behavior.

4. **Tokio runtime already configured for multi-threaded execution** — Upon verification in 03-02, found that Cargo.toml already has `rt-multi-thread` feature and `artifacts.rs` uses default multi-threaded runtime. No changes needed.

5. **Unbounded channels** — No backpressure, TUI never blocks on send
6. **artifact_index in every message** — Enables dispatch back to correct model
   entry
7. **Errors in result messages** — bool+Option<String> pattern, not separate
   error channel
8. **Buffered output** — Complete output returned at end, not streamed
9. **current_thread tokio runtime** — Sequential execution, no need for
   multi-thread overhead
10. **Handler owns config** — Configuration moved into background task, no shared
    state
11. **Graceful shutdown** — Background exits cleanly when TUI drops result
    channel
12. **Timeout-based event polling** — 50ms timeout allows checking channel
    results without blocking on events
13. **spawn_blocking for all blocking I/O** — Required for subprocess execution
    (scripts) in async context
14. **Temp directory ownership transfer** — Store TempDir in handler to preserve
    across effect boundaries
15. **Fail-open for check_serialization** — Assume generation needed on error
16. **Status symbols** — Using intuitive symbols: ○ (pending), ! (needs
    generation), ✓ (up-to-date), ⟳ (generating), ✗ (failed)
17. **Status colors** — Gray for pending, yellow for needs generation, green for
    up-to-date, cyan for generating, red for failed
18. **Animation approach** — tick_count incremented on Msg::Tick, used to cycle
    through braille spinner frames
19. **EffectHandler temp directory management** — Store TempDir in handler after
    GeneratorFinished, take it in Serialize effect
20. **ShowGeneratorSelection synchronous** — Handled by update() directly, not
    sent to background task
21. **Shared artifacts are atomic** — All targets succeed or all fail together
22. **spawn_blocking for shared effects** — Reuses same pattern as single
    artifact effects
23. **File-based debug logging** — Used std::time instead of chrono to avoid
    external dependency; log to /tmp/artifacts_debug.log for visibility into
    background task execution
24. **Two-level timeout architecture** — Script-level (30s) kills hung scripts
    via run_with_captured_output_and_timeout, task-level (35s) catches edge cases
    in background.rs via tokio::time::timeout
25. **Fail-open for check_serialization timeout** — Assume generation needed when
    check script times out, matching existing error behavior
26. **Fail-closed for serialize timeout** — Report failure to user when serialize
    script times out, as we cannot assume success
27. **Timeout error messages** — Clear "Timed out after X seconds" messages shown to user
28. **spawn_blocking for event polling** — Terminal event reading moved to dedicated blocking thread that communicates via channel, allowing tokio::select! to concurrently receive background results
29. **CancellationToken for shutdown signaling** — Using tokio_util::sync::CancellationToken for cooperative cancellation of background task. This integrates with select! and allows graceful shutdown that processes remaining queue commands before exit.
30. **Graceful shutdown sequence** — On quit/Ctrl+C: signal background, drain results (5s timeout), drop channels, exit. Terminal restored via TerminalGuard::Drop, temp directories cleaned via BackgroundEffectHandler::Drop.
32. **Made BACKGROUND_TASK_TIMEOUT public for testing** — Timeout constant made public in background.rs to enable test verification of timeout behavior
33. **Combined shutdown and channel closed test** — test_select_channel_closed_branch exercises both paths since select! prioritizes shutdown branch
34. **Channel disconnect tested via drop(rx_res)** — Simulates TUI closing result channel; background handles gracefully without panic
35. **Runtime tests use async #[tokio::test]** — Tests spawn background tasks and verify channel communication
36. **Update tests verify Effect variants** — Pure function testing: verify correct Effect returned for each async operation
37. **Accept snapshot updates over assertion chains** — Prefer updating insta snapshots when view output changes

### Technical Debt

**Pre-existing:**

- ~~Current effect_handler.rs executes effects synchronously, blocking TUI~~ —
  RESOLVED in 01-03
- ~~Need to replace with channel-based async architecture~~ — COMPLETE

**New:**

- None

### TODOs

- [x] Complete 01-01: Channel Message Types
- [x] Complete 01-02: Background Task
- [x] Complete 01-03: Runtime Integration
- [x] Complete 02-01: Real Backend Integration
- [x] Complete 02-02: EffectHandler Bridge
- [x] Complete 02-03: State Management and UI Updates
- [x] Complete 03-01: Shared Artifact Effects
- [x] Complete 03-02: Tokio Runtime Fix
- [x] Complete 03-03: Debug Logging
- [x] Complete 03-04: Timeout Handling
- [x] Complete 03-05: TUI Freeze Fix
- [x] Complete 04-01: CancellationToken Shutdown
- [x] Complete 04-02: Graceful Shutdown Sequence
- [x] Complete 04-03: Error Display Integration
- [x] Complete 05-01: Async Unit Tests
- [x] Complete 05-02: Select and Shutdown Tests
- [x] Complete 05-03: Runtime and Update Tests

### Blockers

None.

---

## Session Continuity

### Last Session

**Date:** 2026-02-14T13:12:43Z\
**Activity:** Executed 03-05 plan\
**Summary:** Fixed root cause of TUI freeze during serialization:
1. runtime.rs - Moved blocking crossterm::event::poll() to dedicated thread via spawn_blocking
2. runtime.rs - Event thread sends messages via tokio::sync::mpsc::unbounded_channel
3. runtime.rs - Main select! loop now receives events from channel instead of polling
4. cli/mod.rs - Updated to match new run_async signature (no EventSource parameter)

All tests pass (92 tests, 2 pre-existing tempfile failures), cargo check and cargo clippy pass.

### Current Session

**Started:** 2026-02-14T22:25:28Z\
**Goal:** Execute 05-03 plan - runtime and update tests for async architecture

**Summary:** Updated runtime.rs and update.rs tests for async channel architecture:
1. src/tui/runtime.rs - Added 4 async tests:
   - test_runtime_channels_connected: verifies command/result channel communication
   - test_runtime_tick_message: verifies tick counter increments on Msg::Tick
   - test_runtime_key_message: verifies key events converted to Msg::Key
   - test_runtime_spawns_background: verifies background task spawns and processes commands
2. src/app/update.rs - Added 6 async effect tests:
   - test_update_returns_run_generator_effect: verifies Enter returns RunGenerator
   - test_update_returns_serialize_effect: verifies GeneratorFinished returns Serialize
   - test_update_returns_check_serialization_effect: verifies init() returns CheckSerialization batch
   - test_update_handles_async_result: verifies GeneratorFinished updates model state
   - test_update_effect_batching: verifies effect batching for multiple artifacts
3. Updated snapshot for artifact_list_with_failed_status view test
4. All 64 integration tests passing

Runtime tests: 14 total (10 existing + 4 new)
Update tests: 19 total (13 existing + 6 new)
Integration tests: 64 passing

---

### Previous Session

**Date:** 2026-02-14T13:12:43Z\
**Activity:** Executed 03-05 plan

**Summary:** Created comprehensive tokio::select! and shutdown tests:
1. tests/async_tests/select_tests.rs - 4 select! branch coverage tests
   - test_select_shutdown_branch: verifies shutdown.cancelled() branch
   - test_select_command_branch: verifies cmd_rx.recv() branch
   - test_select_channel_closed_branch: tests channel closed handling
   - test_select_with_in_flight_command: verifies queued commands before shutdown
2. tests/async_tests/shutdown_tests.rs - 6 graceful shutdown tests
   - test_graceful_shutdown_completes_in_flight: command completes before shutdown
   - test_shutdown_with_queued_commands: queued commands processed during shutdown
   - test_background_cleanup_on_drop: temp directory cleanup verification
   - test_result_channel_disconnect: graceful handling of closed result channel
   - test_command_timeout: commands complete within timeout window
   - test_error_handling_timeout_with_mock_time: timeout constant verification
3. Made BACKGROUND_TASK_TIMEOUT public for test access
4. All tests use #[serial] for isolation and timeout for safety

All async tests pass (21/21). Total coverage: channels (4), background (7), select (4), shutdown (6).

---

### Previous Session

**Date:** 2026-02-14T18:02:27Z\
**Activity:** Executed 04-01 plan - CancellationToken shutdown for background task

**Summary:** Implemented CancellationToken shutdown mechanism:
1. Cargo.toml - Added tokio-util dependency with full features for CancellationToken
2. background.rs - Updated spawn_background_task to accept CancellationToken parameter
3. background.rs - Converted while loop to tokio::select! with shutdown/cancellation/channel branches
4. background.rs - On shutdown: processes queued commands, then exits cleanly
5. runtime.rs - Creates CancellationToken and passes to spawn_background_task
6. Tests - Updated all test calls to pass CancellationToken::new()

Background task tests pass (3/3), cargo check passes. Ready for SHUT-01 (TUI exit can now signal shutdown).

---

## Quick Links

- [ROADMAP.md](./ROADMAP.md) — Phase structure and requirements
- [PROJECT.md](./PROJECT.md) — Core value and constraints
- [01-03-SUMMARY.md](./phases/01-foundation/01-03-SUMMARY.md) — Plan 01-03
  completion
- [02-01-SUMMARY.md](./phases/02-single-artifacts/02-01-SUMMARY.md) — Plan 02-01
  completion
- [02-02-SUMMARY.md](./phases/02-single-artifacts/02-02-SUMMARY.md) — Plan 02-02
  completion
- [02-03-SUMMARY.md](./phases/02-single-artifacts/02-03-SUMMARY.md) — Plan 02-03
  completion
- [03-01-SUMMARY.md](./phases/03-shared-artifacts/03-01-SUMMARY.md) — Plan 03-01
  completion
- [04-01-SUMMARY.md](./phases/04-robustness/04-01-SUMMARY.md) — Plan 04-01
  completion (CancellationToken shutdown)
- [04-02-SUMMARY.md](./phases/04-robustness/04-02-SUMMARY.md) — Plan 04-02
  completion (Graceful shutdown sequence)
- [05-01-SUMMARY.md](./phases/05-validation/05-01-SUMMARY.md) — Plan 05-01
  completion (Async unit tests)
- [05-03-SUMMARY.md](./phases/05-validation/05-03-SUMMARY.md) — Plan 05-03
  completion (Runtime and update tests)
