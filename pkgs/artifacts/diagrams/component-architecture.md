# Component Architecture

This diagram shows the main components of the artifacts CLI and how they
interact with each other.

## Overview

The system follows an Elm Architecture pattern with a clear separation between:

1. **Configuration** - Loading and parsing backend.toml and flake.nix
2. **TUI** - Terminal UI with Model-Update-View pattern
3. **Runtime** - Async event loop coordinating foreground and background tasks
4. **Backend** - Script execution for generation, serialization, and checking

## Diagram

```
┌────────────────────────────────────────────────────────────────────────────┐
│                              CONFIGURATION                                 │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   ┌──────────────────────┐          ┌──────────────────────┐               │
│   │   backend.toml       │          │   flake.nix          │               │
│   │   (Backend Config)   │          │   (Artifact Defs)    │               │
│   └──────────┬───────────┘          └──────────┬───────────┘               │
│              │                                 │                           │
│              ▼                                 ▼                           │
│   ┌──────────────────────┐          ┌──────────────────────┐               │
│   │ BackendConfiguration │          │  MakeConfiguration   │               │
│   │   (src/config/)      │          │  (src/config/)       │               │
│   └──────────────────────┘          └──────────────────────┘               │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                              TUI (Elm Architecture)                        │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   ┌─────────────┐     ┌─────────────┐     ┌──────────────┐                 │
│   │   Events    │────▶│    Model    │────▶│   Effect     │                 │
│   │  (Message)  │     │   (State)   │     │ (side-effect)│                 │
│   └─────────────┘     └──────┬──────┘     └──────┬───────┘                 │
│                              │                   │                         │
│                              ▼                   │                         │
│   ┌──────────────┐    ┌─────────────┐            │                         │
│   │    Views     │◀───│   Update    │◀───────────┘                         │
│   │  (Render)    │    │  (update()) │                                      │
│   └──────────────┘    └─────────────┘                                      │
│                                                                            │
│   ┌─────────────────────────────────────────────────────────────────┐      │
│   │                          SCREENS                                │      │
│   ├─────────────────────────────────────────────────────────────────┤      │
│   │  ArtifactList  │  SelectGenerator   │  Prompt  │  Generating    │      │
│   │  ConfirmRegen  │  ChronologicalLog  │  Done                     │      │
│   └─────────────────────────────────────────────────────────────────┘      │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘

                              │
                              ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                              RUNTIME                                       │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   ┌─────────────────────┐         ┌───────────────────────┐                │
│   │  Foreground Loop    │         │  Background Task      │                │
│   │  (tui/runtime.rs)   │         │  (tui/background.rs)  │                │
│   │                     │         │                       │                │
│   │  - Render UI        │◄───────▶│  - Execute Effects    │                │
│   │  - Handle Keys      │  Msgs   │  - Run Scripts        │                │
│   │  - Update State     │  Effects│  - Check Serialization│                │
│   └─────────────────────┘         └───────────────────────┘                │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────────────────────┐
│                              BACKEND                                       │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   ┌─────────────────┐  ┌──────────────────┐  ┌─────────────────┐           │
│   │   Generator     │  │  Serialization   │  │   Helpers       │           │
│   │  (generator.rs) │  │(serialization.rs)│  │  (helpers.rs)   │           │
│   │                 │  │                  │  │                 │           │
│   │ - Run script    │  │ - check_serial   │  │ - Path resolve  │           │
│   │ - Verify output │  │ - serialize      │  │ - Validation    │           │
│   │ - Bubblewrap    │  │ - deserialize    │  │ - Escaping      │           │
│   └────────┬────────┘  └────────┬─────────┘  └─────────────────┘           │
│            │                    │                                          │
│            └──────────┬─────────┘                                          │
│                       ▼                                                    │
│            ┌──────────────────────┐                                        │
│            │   Output Capture     │                                        │
│            │  (output_capture.rs) │                                        │
│            │                      │                                        │
│            │ - stdout/stderr      │                                        │
│            │ - timeout support    │                                        │
│            └──────────────────────┘                                        │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

## Component Descriptions

### Configuration Layer (`src/config/`)

- **backend.rs**: Parses `backend.toml` to get backend scripts and settings
- **make.rs**: Parses JSON from `flake.nix` evaluation to get artifact
  definitions
- **nix.rs**: Helper functions for evaluating Nix expressions

### TUI Layer (`src/app/` + `src/tui/`)

The TUI follows the Elm Architecture:

- **Model** (`app/model.rs`): Immutable application state
- **Message** (`app/message.rs`): Events that trigger state changes
- **Effect** (`app/effect.rs`): Descriptions of side effects to execute
- **Update** (`app/update.rs`): Pure function
  `(Model, Message) -> (Model, Effect)`
- **Views** (`tui/views/`): Render functions `(&Model) -> Frame`

### Runtime (`src/tui/runtime.rs`, `src/tui/background.rs`)

The runtime coordinates:

1. **Foreground loop**: Renders UI, handles keyboard input, updates state
2. **Background task**: Executes effects (check, generate, serialize) in
   parallel
3. **Channel communication**: Messages flow between foreground and background
   via Tokio channels

### Backend (`src/backend/`)

- **generator.rs**: Runs generator scripts with bubblewrap isolation
- **serialization.rs**: Runs check, serialize, deserialize scripts
- **output_capture.rs**: Captures stdout/stderr from script execution
- **helpers.rs**: Utility functions for path resolution and validation
