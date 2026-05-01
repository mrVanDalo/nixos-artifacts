# Artifact State Machine

This diagram tracks the lifecycle of an artifact's `ArtifactStatus` — the
per-artifact state that drives the right pane and decides what action a key
press performs.

The TUI's `Screen` enum is a separate, smaller concern (covered at the bottom).

## Status overview

| Status            | Symbol | Description                                                     |
| ----------------- | ------ | --------------------------------------------------------------- |
| `Pending`         | ○      | Initial state, backend `check` has not run yet                  |
| `NeedsGeneration` | !      | Artifact missing or stale in backend storage                    |
| `UpToDate`        | ✓      | Artifact exists in backend storage and is current               |
| `Generating(_)`   | ⟳      | Pipeline running; inner `GeneratingSubstate` tracks step/output |
| `Failed { … }`    | ✗      | Check, generator, or serialize step failed; can retry           |
| `Cancelled { … }` | ⊘      | User cancelled an in-flight generator (Esc-Esc chord)           |

## Lifecycle

```
                   ┌──────────────────┐
                   │     Pending      │   init() emits CheckSerialization
                   │       (○)        │   for every artifact at startup
                   └────────┬─────────┘
                            │
                            │  CheckFinished
                            ▼
                ┌───────────────────────┐
                │                       │
        ┌───────┤  CheckSerialization   ├──────┐
        │       │   (background task)   │      │
        │       │                       │      │
        │       └───────────┬───────────┘      │
        │                   │                  │
        │ needs_generation  │ up_to_date       │ error
        ▼                   ▼                  ▼
┌────────────────┐  ┌────────────────┐  ┌───────────────┐
│ NeedsGeneration│  │   UpToDate     │  │    Failed     │
│      (!)       │  │     (✓)        │  │     (✗)       │
└───────┬────────┘  └───────┬────────┘  └───────┬───────┘
        │                   │                   │
        │ Enter             │ Enter             │ Enter (retry)
        │ (or 'a' queues)   │  ↓                │
        │                   │ ConfirmRegenerate │
        │                   │   dialog          │
        │                   │ Regenerate ──┐    │
        │                   │              │    │
        ▼                   ▼              ▼    ▼
   ┌──────────────────────────────────────────────────┐
   │  prompts? ── yes ──▶ collect via Model.active_prompt
   │     │                       │
   │     │ no / submitted        │
   │     ▼                       ▼
   │              RunGenerator (Effect)
   └──────────────────┬───────────────────────────────┘
                      │ generator started
                      ▼
            ┌────────────────────────┐
            │  Generating(substate)  │
            │           (⟳)          │
            │  step = Check |        │
            │         Generate |     │
            │         Serialize      │
            └───────────┬────────────┘
                        │
   ┌────────────────────┼─────────────────────┐
   │                    │                     │
   │ all steps OK       │ step failed         │ Esc-Esc / Effect::CancelQueue
   ▼                    ▼                     ▼
┌──────────────┐   ┌──────────────┐    ┌──────────────────┐
│   UpToDate   │   │    Failed    │    │    Cancelled     │
│      (✓)     │   │     (✗)      │    │       (⊘)        │
└──────────────┘   └──────┬───────┘    └─────────┬────────┘
                          │                      │
                          │ Enter (retry)        │ Enter (retry)
                          └──────────────────────┘
                                     │
                                     ▼
                        (re-enters Generating)
```

## Pipeline / 'a' flow

The `'a'` keybind generates **all** artifacts. It does _not_ run them in
parallel. Instead it:

1. Enqueues a `RunGenerator` effect per artifact onto `Model.pipeline_queue`.
2. Drains one effect at a time into `Model.in_flight`.
3. Waits for that artifact's `gen → serialize` cycle to finish before popping
   the next one. Pattern: `gen0 → ser0 → gen1 → ser1 → …` (not
   `gen0 → gen1 → … → ser0 → ser1 → …`).
4. On `Esc-Esc` (held within 500 ms), `Effect::CancelQueue` drains the pending
   queue and signals the in-flight generator's process group (SIGTERM, then
   SIGKILL). Any artifact already running transitions to `Cancelled`; queued
   ones reset to `NeedsGeneration`.

Backends always run sequentially — generators are dispatched through one FIFO
channel in `background.rs`. The frontend may emit individual effects or an
`Effect::Batch` (flattened by `runtime.rs`); the end result is the same.

## Screen transitions

`Screen` selects which view dispatcher renders. Inline UI (prompts, generation
progress) is **not** a screen.

| Screen              | Entry                                    | Exit                 |
| ------------------- | ---------------------------------------- | -------------------- |
| `ArtifactList`      | Default                                  | `q`, navigation, `l` |
| `SelectGenerator`   | Shared artifact with multiple generators | Enter (select), Esc  |
| `ConfirmRegenerate` | Enter on `UpToDate` artifact             | Regenerate / Leave   |
| `Done`              | After all artifacts processed (auto)     | Any key              |
| `ChronologicalLog`  | `l` from `ArtifactList`                  | Esc                  |

Inline UI on `ArtifactList`:

- Prompt input: `Model.active_prompt: Option<PromptState>`. When set, key events
  route to the prompt handler and `selected_index` is locked to
  `active_prompt.artifact_index`.
- Generation progress: rendered in the right pane from the selected artifact's
  `ArtifactStatus::Generating(GeneratingSubstate)` — current step and
  accumulated stdout/stderr.

## Key code

- `app/model/artifact.rs` — `ArtifactStatus` enum and `GeneratingSubstate`.
- `app/model/core.rs` — `Screen` enum, `Model.active_prompt`,
  `Model.pipeline_queue`, `Model.in_flight`.
- `app/update/mod.rs` — main `update()`; pipeline draining at
  `pipeline_queue.pop_front()`.
- `app/update/artifact_list.rs` — `'a'` enqueue, Enter handling.
- `app/update/generating.rs` — clears the queue on cancel.
- `tui/background.rs` — single FIFO consumer of generator/serialize effects.
