# Artifact State Machine

This diagram shows the different states an artifact can go through and the
transitions between them.

## State Overview

| State           | Symbol | Description                              |
| --------------- | ------ | ---------------------------------------- |
| Pending         | ○      | Initial state, check not yet run         |
| NeedsGeneration | !      | Artifact missing/stale, needs generation |
| UpToDate        | ✓      | Artifact exists and is current           |
| Generating      | ⟳      | Currently running generation pipeline    |
| Failed          | ✗      | Generation failed, can retry             |

## Diagram

```
                         ┌──────────────────┐
                         │                  │
                         │     PENDING      │  ◀── Initial state
                         │   (○ symbol)     │      (before check)
                         │                  │
                         └────────┬─────────┘
                                  │
                    ┌─────────────┴───────────┐
                    │                         │
                    │  init() creates         │
                    │  CheckSerialization     │
                    │  effect for all         │
                    │  pending artifacts      │
                    ▼                         │
         ┌──────────────────────┐             │
         │                      │             │
         │  CheckSerialization  │             │
         │  (background task)   │             │
         │                      │             │
         └──────────┬───────────┘             │
                    │                         │
          ┌─────────┴─────────┐               │
          │                   │               │
          ▼                   ▼               │
┌─────────────────┐  ┌─────────────────┐      │
│                 │  │                 │      │
│ NEEDS_GENERATION│  │    UP_TO_DATE   │      │
│   (! symbol)    │  │   (✓ symbol)    │      │
│                 │  │                 │      │
│ Artifact does   │  │ Artifact exists │      │
│ NOT exist in    │  │ and is current  │      │
│ backend storage │  │ in backend      │      │
│                 │  │                 │      │
└────────┬────────┘  └────────┬────────┘      │
         │                    │               │
         │                    │   ┌───────────┘
         │                    │   │ User can still
         │                    │   │ trigger regen
         │  User presses      │   │ from UpToDate
         │  Enter (Generate)  │   │
         │                    │   │
         │                    ▼   │
         │           ┌───────────────────────┐
         │           │                       │
         │           │  ConfirmRegenerate    │
         │           │  (confirmation dialog │
         │           │   for existing art.)  │
         │           │                       │
         │           └───────────┬───────────┘
         │                       │
         │                       │ User selects
         │                       │ "Regenerate"
         │                       │
         └───────────┬───────────┘
                     │
                     ▼
         ┌──────────────────────┐
         │                      │
         │     GENERATING       │
         │    (⟳ symbol)        │
         │                      │
         └──────────┬───────────┘
                    │
         ┌──────────┴─────────┬──────────────────┐
         │                    │                  │
         ▼                    ▼                  ▼
┌──────────────────┐  ┌────────────────┐  ┌─────────────────┐
│CheckSerialization│  │RunningGenerator│  │   Serializing   │
│   (step 1)       │  │   (step 2)     │  │    (step 3)     │
└────────┬─────────┘  └──────┬─────────┘  └───────┬─────────┘
         │                   │                    │
         │                   │                    │
         │  skip if          │                    │
         │  already checked  │                    │
         └───────────────────┘                    │
                                                  │
                     ┌────────────────────────────┘
                     │
           ┌─────────┴─────────┐
           │                   │
           ▼                   ▼
┌─────────────────┐   ┌─────────────────┐
│                 │   │                 │
│   UP_TO_DATE    │   │     FAILED      │
│   (✓ symbol)    │   │   (✗ symbol)    │
│                 │   │                 │
│ Success path    │   │ Error path      │
│ generation      │   │ retry_available │
│ complete        │   │ = true          │
│                 │   │                 │
└─────────────────┘   └────────┬────────┘
                               │
                               │ User can retry
                               │ (press Enter)
                               │
                               └──────┬────────┘
                                      │
                                      ▼
                         ┌──────────────────────┐
                         │                      │
                         │    (back to Generate │
                         │     flow from        │
                         │     NEEDS_GENERATION │
                         │     or UpToDate)     │
                         │                      │
                         └──────────────────────┘
```

## State Transitions

### Initial Flow

```
┌─────────┐                   ┌────────────────────┐                    ┌──────────────────┐
│ PENDING │ ──(init Effect)──▶│ CheckSerialization │──(success)────────▶│ UP_TO_DATE       │
│         │                   │ (background task)  │                    │                  │
│         │                   │                    │──(needs gen)──────▶│ NEEDS_GENERATION │
│         │                   │                    │                    │                  │
│         │                   │                    │──(error)──────────▶│ FAILED           │
└─────────┘                   └────────────────────┘                    └──────────────────┘
```

### Generation Flow

```
┌──────────────────┐   (Enter)   ┌───────────────────┐   (no prompts)   ┌────────────────┐
│ NEEDS_GENERATION │ ───────────▶│ Screen::Generating│ ────────────────▶│ RunGenerator   │
│                  │             │                   │                  │ (Effect)       │
└──────────────────┘             └───────────────────┘                  └────────────────┘
                                                                          │
┌──────────────────┐   (Enter)   ┌───────────────────┐   (has prompts)    │
│ UP_TO_DATE       │ ───────────▶│ ConfirmRegenerate │ ───────────────┐   │
│                  │             │ (dialog)          │                │   │
└──────────────────┘             └───────────────────┘                │   │
                                                                      │   │
                                          ┌───────────────────┐◀──────┘   │
                                          │ Screen::Prompt    │           │
                                          │ (collect inputs) │            │
                                          └────────┬─────────┘            │
                                                   │ (submit)             │
                                                   ▼                      │
                                          ┌────────────────┐              │
                                          │ RunGenerator   │◀─────────────┘
                                          │ (Effect)       │
                                          └────────────────┘
                                                   │
                                                   ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                              GENERATING STATE                                       │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                     │
│   GenerationStep::CheckSerialization ──▶ GenerationStep::RunningGenerator           │
│                                                  │                                  │
│                                                  ▼                                  │
│                                    GenerationStep::Serializing                      │
│                                                  │                                  │
│                           ┌──────────────────────┴──────────────────────┐           │
│                           │                                             │           │
│                           ▼                                             ▼           │
│                    ┌─────────────────┐                          ┌──────────────┐    │
│                    │ UP_TO_DATE      │                          │ FAILED       │    │
│                    │ (success)       │                          │ (error)      │    │
│                    └─────────────────┘                          └──────────────┘    │
│                                                                                     │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

## Key Functions

The state transitions are implemented in:

- **`app/update.rs`**: `update()` function handles all state transitions
- **`app/update.rs`**: `init()` creates initial `CheckSerialization` effects
- **`tui/effect_handler.rs`**: Executes effects and returns `Message` results

## Screen Transitions

The `Screen` enum determines what the user sees:

| Screen              | Entry Point                              | Exit Condition               |
| ------------------- | ---------------------------------------- | ---------------------------- |
| `ArtifactList`      | Default                                  | Enter key, 'l' key           |
| `SelectGenerator`   | Shared artifact with multiple generators | Enter (select), Esc (cancel) |
| `ConfirmRegenerate` | Enter on UpToDate artifact               | Regenerate/Leave selection   |
| `Prompt`            | Artifact has prompts                     | Enter (submit), Esc (cancel) |
| `Generating`        | After prompts or generator selection     | Generation complete          |
| `Done`              | After all artifacts processed            | Any key                      |
| `ChronologicalLog`  | 'l' key from list                        | Esc key                      |
