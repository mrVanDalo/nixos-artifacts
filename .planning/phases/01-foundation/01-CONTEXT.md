# Phase 1: Foundation — Core Architecture - Context

**Gathered:** 2025-02-13 **Status:** Ready for planning

<domain>
## Phase Boundary

Establish the channel-based communication system and background job
infrastructure that will power all effect execution. The TUI must remain
responsive while effects (check_serialization, generator, serialize scripts) run
in a separate async task.

</domain>

<decisions>
## Implementation Decisions

### Channel Design

- **Message types:** Separate per-effect variants — `CheckSerialization`,
  `GeneratorFinished`, `SerializeFinished`, `SharedCheckSerialization`,
  `SharedGeneratorFinished`, `SharedSerializeFinished`
- **Channel capacity:** Unbounded (no backpressure, TUI never blocks)
- **Message content:** Include artifact ID in every message for dispatch context
- **Result structure:** Claude's discretion — follow Rust/Ratatui conventions
- **Script output:** Complete output returned at end of execution (buffered, not
  streamed)

### Directionality

- **Foreground → Background:** `Effect` messages (what to execute)
- **Background → Foreground:** `EffectResult` messages (outcomes)
- **Sequential processing:** Effects execute FIFO in single background task

### Error Handling

- **Result encapsulation:** Errors travel in result messages, not separate error
  channel
- **Claude's discretion:** Specific error variant design

</decisions>

<specifics>
## Specific Ideas

- Uses existing tokio dependency from ratatui — no new dependencies
- Single background task spawned at TUI startup, not per-effect
- State isolation: Model only touched by TUI foreground thread
- Background receives immutable data needed for execution, sends back results

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

_Phase: 01-foundation_ _Context gathered: 2025-02-13_
