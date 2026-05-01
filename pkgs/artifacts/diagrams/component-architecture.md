# Component Architecture

This diagram shows the main components of the artifacts CLI and how they
interact.

## Overview

The system follows the Elm Architecture pattern with a clear separation between:

1. **Configuration** — Loading and parsing `backend.toml` and the `make` JSON
   produced from `flake.nix`.
2. **TUI** — Terminal UI with Model-Update-View, plus inline UI state.
3. **Runtime** — Async loop coordinating the foreground render loop with a
   single FIFO background task.
4. **Backend** — Script execution for the `check` and `serialize` steps, plus
   the bubblewrapped generator.

## Diagram

```
┌────────────────────────────────────────────────────────────────────────────┐
│                              CONFIGURATION                                 │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   ┌──────────────────────┐          ┌──────────────────────┐               │
│   │   backend.toml       │          │   make.json          │               │
│   │   (Backend scripts   │          │   (Extracted from    │               │
│   │    per target)       │          │    flake.nix opts)   │               │
│   └──────────┬───────────┘          └──────────┬───────────┘               │
│              ▼                                 ▼                           │
│   ┌──────────────────────┐          ┌──────────────────────┐               │
│   │ BackendConfiguration │          │  MakeConfiguration   │               │
│   │ (config/backend.rs)  │          │  (config/make.rs)    │               │
│   └──────────────────────┘          └──────────────────────┘               │
│                                                                            │
└──────────────────────────────────────┬─────────────────────────────────────┘
                                       ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                          TUI (Elm Architecture)                            │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   ┌─────────────┐     ┌─────────────┐     ┌──────────────┐                 │
│   │   Events    │────▶│    Model    │────▶│   Effect     │                 │
│   │ (Message)   │     │   (state)   │     │ (side-effect)│                 │
│   └─────────────┘     └──────┬──────┘     └──────┬───────┘                 │
│                              │                   │                         │
│                              ▼                   │                         │
│   ┌──────────────┐    ┌─────────────┐            │                         │
│   │    Views     │◀───│   update()  │◀───────────┘                         │
│   │  (render)    │    │  (pure fn)  │                                      │
│   └──────────────┘    └─────────────┘                                      │
│                                                                            │
│   ┌──────────────────────────────────────────────────────────────────┐     │
│   │                           SCREENS (Screen enum)                  │     │
│   │   ArtifactList │ SelectGenerator │ ConfirmRegenerate │ Done      │     │
│   │   ChronologicalLog                                               │     │
│   └──────────────────────────────────────────────────────────────────┘     │
│                                                                            │
│   ┌──────────────────────────────────────────────────────────────────┐     │
│   │   INLINE UI on ArtifactList (Model fields, NOT Screens)          │     │
│   │ • active_prompt: Option<PromptState>  — inline prompt input      │     │
│   │   when Some, key events route to prompt handler;                 │     │
│   │   selected_index is locked to active_prompt.artifact_index       │     │
│   │ • Per-artifact ArtifactStatus::Generating(GeneratingSubstate)    │     │
│   │   drives the right pane (current step + accumulated output)      │     │
│   │ • pipeline_queue + in_flight  — drive the 'a' generate-all flow  │     │
│   └──────────────────────────────────────────────────────────────────┘     │
│                                                                            │
└──────────────────────────────────────┬─────────────────────────────────────┘
                                       ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                              RUNTIME                                       │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   ┌─────────────────────┐   cmd_tx   ┌────────────────────────────┐        │
│   │  Foreground loop    │ ─────────▶ │  Background task (FIFO)    │        │
│   │  (tui/runtime.rs)   │            │  (tui/background.rs)       │        │
│   │                     │ ◀───────── │                            │        │
│   │  - Render UI        │  result_tx │  - Sequential consumer of  │        │
│   │  - Handle keys      │            │    Effect commands         │        │
│   │  - Run update()     │            │  - One generator at a time │        │
│   │  - Flatten          │            │  - Emits Msg::*Finished    │        │
│   │    Effect::Batch    │            │    keyed by artifact_index │        │
│   └────────┬────────────┘            └────────────────────────────┘        │
│            │                                                               │
│            │ cancel_tx (separate channel)                                  │
│            ▼                                                               │
│   ┌─────────────────────┐                                                  │
│   │ CancelSignal stream │  Esc-Esc / Effect::CancelQueue                   │
│   │ - Drains queued     │  drains pipeline_queue and signals the           │
│   │   effects           │  in-flight generator's process group             │
│   │ - SIGTERM→SIGKILL   │  (SIGTERM, then SIGKILL).                        │
│   └─────────────────────┘                                                  │
│                                                                            │
└──────────────────────────────────────┬─────────────────────────────────────┘
                                       ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                              BACKEND                                       │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   ┌─────────────────┐  ┌──────────────────┐  ┌─────────────────┐           │
│   │   Generator     │  │  Serialization   │  │   Helpers       │           │
│   │ (generator.rs)  │  │(serialization.rs)│  │  (helpers.rs)   │           │
│   │                 │  │                  │  │                 │           │
│   │ - Run script    │  │ - run_check_…    │  │ - Path resolve  │           │
│   │   in bubblewrap │  │ - run_serialize  │  │ - Validation    │           │
│   │ - Verify output │  │ - shared_*       │  │ - Escaping      │           │
│   │   files         │  │   variants       │  │                 │           │
│   └────────┬────────┘  └────────┬─────────┘  └─────────────────┘           │
│            └──────────┬─────────┘                                          │
│                       ▼                                                    │
│            ┌──────────────────────┐                                        │
│            │   Output Capture     │                                        │
│            │  (output_capture.rs) │                                        │
│            │ - stdout/stderr      │                                        │
│            │ - timeout support    │                                        │
│            │ - process-group kill │                                        │
│            └──────────────────────┘                                        │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

## Component descriptions

### Configuration layer (`src/config/`)

- `backend.rs` — parses `backend.toml`. Each backend has nested
  `[<name>.nixos]`, `[<name>.home]`, and optional `[<name>.shared]` sections
  with `check` and `serialize` script paths.
- `make.rs` — parses the `make.json` produced from `flake.nix` evaluation;
  builds `ArtifactDef`/`SharedArtifactInfo` records.

### TUI layer (`src/app/` + `src/tui/`)

- `app/model/` — immutable `Model`, the `Screen` enum, `ArtifactStatus`,
  `PromptState`, etc.
- `app/message.rs` — input events (`Msg::Key`, `Msg::CheckFinished`,
  `Msg::GeneratorFinished`, `Msg::SerializeFinished`, …).
- `app/effect.rs` — descriptions of side effects (`Effect::CheckSerialization`,
  `Effect::RunGenerator`, `Effect::Serialize`, `Effect::CancelQueue`,
  `Effect::Batch`).
- `app/update/` — the pure `update()` function, split per screen/feature.
- `tui/views/` — ratatui renderers, dispatched by `Screen`.

### Runtime (`src/tui/runtime.rs`, `src/tui/background.rs`)

- Foreground loop renders the UI, polls events, runs `update()`, and dispatches
  effects. `Effect::Batch` is flattened into individual commands here (it must
  not reach the background task — `BackgroundEffectHandler::execute` panics if
  it does, by design).
- Background task is a single FIFO consumer. Generators **always** run
  sequentially. Each command produces exactly one `Msg::*Finished` keyed by
  `artifact_index`.
- A separate cancel channel carries `Effect::CancelQueue`, draining queued
  effects and killing the in-flight generator's process group.

### Backend (`src/backend/`)

- `generator.rs` — runs the user's generator inside `bubblewrap` for
  filesystem/network isolation, then verifies that the declared files were
  produced.
- `serialization.rs` — runs the backend's `check` and `serialize` scripts (plus
  `shared_*` variants), passing artifact metadata via environment variables and
  a `targets.json` file.
- `output_capture.rs` — captures stdout/stderr, applies the per-step timeout,
  and kills the entire process group on cancel.
- `helpers.rs` — path resolution, script validation, shell escaping.
