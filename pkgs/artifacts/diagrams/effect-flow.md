# Effect Flow

This diagram shows how effects are created, dispatched, executed, and how
results flow back into the application state.

## Overview

Effects are side-effect descriptions (data) emitted by the pure `update()`
function. The runtime owns the actual execution.

The pipeline has two channels:

- **`cmd_tx`** — main FIFO of effect commands consumed sequentially by the
  background task.
- **`cancel_tx`** — separate signal channel used by `Effect::CancelQueue` to
  drain the queue and kill the in-flight generator's process group.

## Diagram

```
┌────────────────────────────────────────────────────────────────────────────┐
│                          EFFECT EXECUTION FLOW                             │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   USER INPUT  /  TIMER  /  RESULT MESSAGE                                  │
│       │                                                                    │
│       ▼                                                                    │
│   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐                  │
│   │   Message   │────▶│   update()  │────▶│   Effect    │                  │
│   │             │     │ (pure func) │     │   (data)    │                  │
│   └─────────────┘     └─────────────┘     └──────┬──────┘                  │
│                                                  │                         │
│                                                  ▼                         │
│   ┌──────────────────────────────────────────────────────────────────┐     │
│   │                    RUNTIME (foreground)                          │     │
│   │                                                                  │     │
│   │   effect_to_command():                                           │     │
│   │     • Effect::None     ──▶ ()                                    │     │
│   │     • Effect::Quit     ──▶ exit                                  │     │
│   │     • Effect::Batch    ──▶ flattened into individual commands;   │     │
│   │                            MUST NOT reach background task        │     │
│   │     • Effect::CancelQueue ──▶ cancel_tx (separate channel)       │     │
│   │     • All other effects   ──▶ cmd_tx (main FIFO)                 │     │
│   └────────────────────┬─────────────────────────────────────────────┘     │
│                        │                                                   │
│                        ▼                                                   │
│   ┌──────────────────────────────────────────────────────────────────┐     │
│   │                BACKGROUND TASK (single FIFO consumer)            │     │
│   │                background.rs ~lines 888-952                      │     │
│   │                                                                  │     │
│   │   Generators ALWAYS run sequentially. Each effect produces       │     │
│   │   exactly one Msg::*Finished keyed by artifact_index.            │     │
│   │                                                                  │     │
│   │   Effect::CheckSerialization {..}                                │     │
│   │      └──▶ run_check_serialization() ──▶ Msg::CheckFinished       │     │
│   │                                                                  │     │
│   │   Effect::RunGenerator {..}                                      │     │
│   │      └──▶ run_generator_script() (in bubblewrap)                 │     │
│   │           │  process group is kill-target on cancel              │     │
│   │           ▼                                                      │     │
│   │           verify_generated_files() ──▶ Msg::GeneratorFinished    │     │
│   │                                                                  │     │
│   │   Effect::Serialize {..}                                         │     │
│   │      └──▶ run_serialize() ──▶ Msg::SerializeFinished             │     │
│   │                                                                  │     │
│   │   Shared* variants (SharedCheck, SharedRunGenerator,             │     │
│   │   SharedSerialize) — handled identically, route to               │     │
│   │   run_shared_* helpers in serialization.rs                       │     │
│   │                                                                  │     │
│   │   Cancel signal (separate channel):                              │     │
│   │   cancel_tx ──▶ drains pipeline_queue inside the runtime,        │     │
│   │                 then SIGTERMs the in-flight generator's          │     │
│   │                 process group, escalating to SIGKILL.            │     │
│   │                 Affected artifact transitions to                 │     │
│   │                 ArtifactStatus::Cancelled.                       │     │
│   └────────────────────┬─────────────────────────────────────────────┘     │
│                        │ result_tx                                         │
│                        ▼                                                   │
│   ┌──────────────────────────────────────────────────────────────────┐     │
│   │                    RUNTIME LOOP (foreground)                     │     │
│   │   1. Poll user input (non-blocking)                              │     │
│   │   2. Drain pending Msg::*Finished from result_tx                 │     │
│   │   3. update(model, msg) → (model, effect)                        │     │
│   │   4. Dispatch the new effect (back to top)                       │     │
│   │   5. Render UI from current Model                                │     │
│   │   6. Loop until quit                                             │     │
│   └──────────────────────────────────────────────────────────────────┘     │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

## 'a' (generate-all) pipeline

The `'a'` keybind enqueues a `RunGenerator` effect onto `Model.pipeline_queue`
for every artifact in `NeedsGeneration`. The runtime drains the queue one
artifact at a time and waits for that artifact's `gen → serialize` cycle to
finish before popping the next entry. The order is
`gen0 → ser0 → gen1 →
ser1 → …`, **not** `gen0 → gen1 → … → ser0 → ser1 → …`.

`Esc-Esc` (held within 500 ms) emits `Effect::CancelQueue`, which:

1. Drops everything in `pipeline_queue` (queued artifacts revert to
   `NeedsGeneration`).
2. Signals the in-flight generator's process group (SIGTERM, then SIGKILL).
3. Marks the in-flight artifact as `ArtifactStatus::Cancelled`.

## Why a single FIFO?

It is a deliberate design choice — see `background.rs:888-952`. The frontend may
dispatch via `Effect::Batch` or one-by-one; either way the runtime flattens to
individual commands and the background task consumes them sequentially. There is
no batched response message; each command yields one `Msg::*Finished` keyed by
`artifact_index`.
