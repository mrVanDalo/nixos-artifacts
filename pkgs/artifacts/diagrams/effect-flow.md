# Effect Flow

This diagram shows how effects are created, executed, and how results flow back
into the application state.

## Overview

Effects are side-effect descriptions (data) that are executed by the background
task. The runtime coordinates between:

1. **Foreground**: UI rendering, user input handling, state updates
2. **Background**: Script execution, file I/O, network operations

## Diagram

```
┌────────────────────────────────────────────────────────────────────────────┐
│                          EFFECT EXECUTION FLOW                             │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   USER INPUT                                                               │
│       │                                                                    │
│       ▼                                                                    │
│   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐                  │
│   │   Message   │────▶│   update()  │────▶│   Effect    │                  │
│   │  (KeyEvent) │     │ (pure func) │     │   (data)    │                  │
│   └─────────────┘     └─────────────┘     └──────┬──────┘                  │
│                                                  │                         │
│                     ┌────────────────────────────┘                         │
│                     ▼                                                      │
│   ┌──────────────────────────────────────────────────────────────────┐     │
│   │                    BACKGROUND TASK                               │     │
│   │                                                                  │     │
│   │   Effect::CheckSerialization {..}                                │     │
│   │       │                                                          │     │
│   │       ▼                                                          │     │
│   │   run_check_serialization() ──▶ Message::CheckSerializationResult│     │
│   │       │                                                          │     │
│   │       └──────────────────────────────┐                           │     │
│   │                                      │                           │     │
│   │   Effect::RunGenerator {..}          │                           │     │
│   │       │                              │                           │     │
│   │       ▼                              │                           │     │
│   │   run_generator_script()             │   Sent back               │     │
│   │       │                              │   to runtime              │     │
│   │       ▼                              │   via channel             │     │
│   │   verify_generated_files()           │                           │     │
│   │       │                              │                           │     │
│   │       └──────────────────────────────┤                           │     │
│   │                                      │                           │     │
│   │   Effect::Serialize {..}             │                           │     │
│   │       │                              │                           │     │
│   │       ▼                              │                           │     │
│   │   run_serialize()                    │                           │     │
│   │       │                              │                           │     │
│   │       └──────────────────────────────┤                           │     │
│   │                                      │                           │     │
│   │   (similar for Shared* variants)     │                           │     │
│   │                                      │                           │     │
│   └──────────────────────────────────────┼───────────────────────────┘     │
│                                          │                                 │
│                                          ▼                                 │
│   ┌─────────────────────────────────────────────────────────────────┐      │
│   │                    RUNTIME LOOP                                 │      │
│   │                                                                 │      │
│   │   1. Drain pending results from channel                         │      │
│   │   2. Update model with each result                              │      │
│   │   3. Execute any follow-up effects                              │      │
│   │   4. Check for user input (non-blocking)                        │      │
│   │   5. Render UI                                                  │      │
│   │   6. Loop until quit                                            │      │
│   │                                                                 │      │
│   └─────────────────────────────────────────────────────────────────┘      │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```
