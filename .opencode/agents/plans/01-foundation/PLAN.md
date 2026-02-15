---
phase: 01-foundation
plan: summary
type: summary
wave: N/A
depends_on: []
files_modified: []
autonomous: true
---

# Phase 1: Foundation — Core Architecture

## Overview

This phase establishes the channel-based communication system and background job
infrastructure that will power all effect execution. The TUI will remain
responsive while effects (check_serialization, generator, serialize scripts) run
in a separate async task.

## User Decisions (from /gsd-discuss-phase)

### Locked Decisions

- **Message types:** Separate per-effect variants for all 6 effect types
- **Channel capacity:** Unbounded (no backpressure)
- **Message content:** Include artifact_index in every message
- **Script output:** Complete output returned at end (buffered, not streamed)
- **Directionality:** Foreground → Background (Effect), Background → Foreground
  (EffectResult)
- **Sequential processing:** FIFO execution in single background task
- **Error handling:** Errors travel in result messages, not separate channel
- **Dependencies:** Uses existing tokio from ratatui (no new deps)

### Claude's Discretion

- Result structure design
- Specific error variant design
- Implementation details of async event source

### Deferred Ideas

- None — discussion stayed within phase scope

## Plan Structure

| Plan  | Objective                    | Tasks | Wave | Depends On   |
| ----- | ---------------------------- | ----- | ---- | ------------ |
| 01-01 | Create channel message types | 3     | 1    | —            |
| 01-02 | Implement background task    | 3     | 1    | —            |
| 01-03 | Refactor runtime for async   | 4     | 2    | 01-01, 01-02 |

## Wave Execution

### Wave 1 (Independent - Can Run in Parallel)

- **Plan 01-01:** Create src/tui/channels.rs with EffectCommand and EffectResult
  enums
- **Plan 01-02:** Create src/tui/background.rs with spawn_background_task
  function

### Wave 2 (Depends on Wave 1)

- **Plan 01-03:** Refactor runtime.rs to use tokio::select!, remove synchronous
  effect_handler.rs

## Files Modified

| File                  | Plans | Purpose                           |
| --------------------- | ----- | --------------------------------- |
| src/tui/channels.rs   | 01-01 | Channel message types             |
| Cargo.toml            | 01-01 | Add tokio dependency              |
| src/tui/background.rs | 01-02 | Background task implementation    |
| src/tui/runtime.rs    | 01-03 | Async runtime with tokio::select! |
| src/tui/events.rs     | 01-03 | AsyncEventSource trait            |
| src/bin/artifacts.rs  | 01-03 | #[tokio::main]                    |
| src/cli/mod.rs        | 01-03 | Async run function                |

## Success Criteria

1. ✅ TUI launches and runs without the old effect_handler.rs file existing
2. ✅ Sending a test message through the channel works end-to-end
3. ✅ Background task is spawned and runs independently
4. ✅ Foreground TUI loop continues processing events while background runs
5. ✅ No synchronous blocking calls remain in the TUI runtime loop

## Must-Haves Summary

### Observable Truths

- User sees TUI remains responsive during generation
- Background operations complete and results appear
- No UI freezing during long-running scripts

### Required Artifacts

- Channel message types (EffectCommand, EffectResult)
- Background task with FIFO execution
- Async runtime with concurrent polling
- Tokio runtime initialization

### Key Links

- Foreground sends EffectCommand → Background receives
- Background sends EffectResult → Foreground receives
- Tokio::select! polls both events and results

## Next Phase

After Phase 1 completes, Phase 2 (Single Artifacts) will migrate the actual
backend operations into the background task.

## How to Execute

```bash
# Execute entire phase
/gsd-execute-phase 01-foundation

# Execute specific plan
/gsd-execute-plan 01-foundation/01-01

# Execute specific wave
/gsd-execute-wave 01-foundation 1
```
