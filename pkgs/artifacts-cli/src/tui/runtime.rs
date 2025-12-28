use crate::app::model::Model;
use crate::app::{Effect, Msg, update};
use crate::tui::events::EventSource;
use crate::tui::views::render;
use anyhow::Result;
use ratatui::{Terminal, backend::Backend};

/// The result of running the TUI application
#[derive(Debug, Clone)]
pub struct RunResult {
    pub final_model: Model,
    pub frames_rendered: usize,
}

/// Trait for executing side effects.
/// This allows injecting different effect handlers for testing vs production.
pub trait EffectHandler {
    /// Execute an effect and return any resulting messages.
    /// The handler may return messages that should be fed back into the update loop.
    fn execute(&mut self, effect: Effect, model: &Model) -> Result<Vec<Msg>>;
}

/// A no-op effect handler that ignores all effects.
/// Useful for pure UI testing where we don't want real side effects.
#[derive(Debug, Default)]
pub struct NoOpEffectHandler;

impl EffectHandler for NoOpEffectHandler {
    fn execute(&mut self, _effect: Effect, _model: &Model) -> Result<Vec<Msg>> {
        Ok(vec![])
    }
}

/// Run the TUI application with the given components.
///
/// This is the main entry point for running the Elm-architecture loop:
/// 1. Render the current model
/// 2. Get the next event
/// 3. Update the model with the event
/// 4. Execute any resulting effects
/// 5. Repeat until quit or events exhausted
pub fn run<B, E, H>(
    terminal: &mut Terminal<B>,
    events: &mut E,
    effects: &mut H,
    mut model: Model,
) -> Result<RunResult>
where
    B: Backend,
    E: EventSource,
    H: EffectHandler,
{
    let mut frames_rendered = 0;

    loop {
        // Render the current state
        terminal.draw(|f| render(f, &model))?;
        frames_rendered += 1;

        // Get the next event
        let Some(msg) = events.next_event() else {
            // Event source exhausted, exit gracefully
            break;
        };

        // Update state with the event
        let (new_model, effect) = update(model, msg);
        model = new_model;

        // Check for quit
        if effect.is_quit() {
            break;
        }

        // Execute effects and process any resulting messages
        // We need to recursively execute effects until there are none left
        let mut pending_effect = effect;
        loop {
            let result_msgs = execute_effect(effects, pending_effect, &model)?;
            if result_msgs.is_empty() {
                break;
            }

            // Process all result messages
            let mut next_effect = Effect::None;
            for msg in result_msgs {
                let (new_model, new_effect) = update(model, msg);
                model = new_model;
                if new_effect.is_quit() {
                    return Ok(RunResult {
                        final_model: model,
                        frames_rendered,
                    });
                }
                // Collect effects to execute (batch them)
                if !new_effect.is_none() {
                    next_effect = new_effect;
                }
            }

            if next_effect.is_none() {
                break;
            }

            // Re-render to show progress
            terminal.draw(|f| render(f, &model))?;
            frames_rendered += 1;

            pending_effect = next_effect;
        }
    }

    Ok(RunResult {
        final_model: model,
        frames_rendered,
    })
}

fn execute_effect<H: EffectHandler>(
    handler: &mut H,
    effect: Effect,
    model: &Model,
) -> Result<Vec<Msg>> {
    match effect {
        Effect::None => Ok(vec![]),
        Effect::Quit => Ok(vec![]),
        Effect::Batch(effects) => {
            let mut all_msgs = vec![];
            for e in effects {
                all_msgs.extend(handler.execute(e, model)?);
            }
            Ok(all_msgs)
        }
        other => handler.execute(other, model),
    }
}

/// Simulate running the app with a scripted event sequence.
/// Returns the final model state after all events are processed.
/// This is useful for testing state transitions without rendering.
pub fn simulate<E: EventSource>(events: &mut E, mut model: Model) -> Model {
    loop {
        let Some(msg) = events.next_event() else {
            break;
        };

        let (new_model, effect) = update(model, msg);
        model = new_model;

        if effect.is_quit() {
            break;
        }
    }

    model
}

/// Simulate running the app and collect the sequence of models.
/// Returns all intermediate states, useful for debugging test failures.
pub fn simulate_with_history<E: EventSource>(events: &mut E, initial: Model) -> Vec<Model> {
    let mut history = vec![initial.clone()];
    let mut model = initial;

    loop {
        let Some(msg) = events.next_event() else {
            break;
        };

        let (new_model, effect) = update(model, msg);
        model = new_model.clone();
        history.push(new_model);

        if effect.is_quit() {
            break;
        }
    }

    history
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model::*;
    use crate::config::make::{ArtifactDef, FileDef, PromptDef};
    use crate::tui::events::ScriptedEventSource;
    use crate::tui::events::test_helpers::*;
    use ratatui::backend::TestBackend;
    use std::collections::BTreeMap;

    fn make_test_artifact(name: &str, prompts: Vec<&str>) -> ArtifactDef {
        let mut prompt_map = BTreeMap::new();
        for p in prompts {
            prompt_map.insert(
                p.to_string(),
                PromptDef {
                    name: p.to_string(),
                    description: Some(format!("Enter {}", p)),
                },
            );
        }
        ArtifactDef {
            name: name.to_string(),
            shared: false,
            files: BTreeMap::from([(
                "test".to_string(),
                FileDef {
                    name: "test".to_string(),
                    path: Some("/test".to_string()),
                    owner: None,
                    group: None,
                },
            )]),
            prompts: prompt_map,
            generator: "/gen".to_string(),
            serialization: "test".to_string(),
        }
    }

    fn make_test_model() -> Model {
        Model {
            screen: Screen::ArtifactList,
            artifacts: vec![
                ArtifactEntry {
                    target: "machine-one".to_string(),
                    target_type: TargetType::Nixos,
                    artifact: make_test_artifact("ssh-key", vec!["passphrase"]),
                    status: ArtifactStatus::Pending,
                },
                ArtifactEntry {
                    target: "machine-two".to_string(),
                    target_type: TargetType::Nixos,
                    artifact: make_test_artifact("api-token", vec![]),
                    status: ArtifactStatus::Pending,
                },
            ],
            selected_index: 0,
            error: None,
        }
    }

    #[test]
    fn test_simulate_navigation() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![
            down(), // Move to second item
            down(), // Stay at second (bottom)
            up(),   // Move back to first
        ]);

        let final_model = simulate(&mut events, model);
        assert_eq!(final_model.selected_index, 0);
    }

    #[test]
    fn test_simulate_quit() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![
            down(),
            char('q'), // Quit
            down(),    // This should not be processed
        ]);

        let final_model = simulate(&mut events, model);
        // Should have moved down once before quitting
        assert_eq!(final_model.selected_index, 1);
    }

    #[test]
    fn test_simulate_with_history() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![down(), down()]);

        let history = simulate_with_history(&mut events, model);

        // Initial + 2 updates = 3 states
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].selected_index, 0);
        assert_eq!(history[1].selected_index, 1);
        assert_eq!(history[2].selected_index, 1); // Stayed at bottom
    }

    #[test]
    fn test_run_with_test_backend() {
        let model = make_test_model();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut events = ScriptedEventSource::new(vec![down(), char('q')]);
        let mut effects = NoOpEffectHandler;

        let result = run(&mut terminal, &mut events, &mut effects, model).unwrap();

        assert_eq!(result.final_model.selected_index, 1);
        assert!(result.frames_rendered >= 2);
    }

    #[test]
    fn test_run_empty_events_exits_gracefully() {
        let model = make_test_model();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut events = ScriptedEventSource::new(vec![]);
        let mut effects = NoOpEffectHandler;

        let result = run(&mut terminal, &mut events, &mut effects, model).unwrap();

        // Should render once then exit
        assert_eq!(result.frames_rendered, 1);
    }

    #[test]
    fn test_enter_prompt_screen() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![
            enter(), // Enter prompt screen for first artifact
        ]);

        let final_model = simulate(&mut events, model);
        assert!(matches!(final_model.screen, Screen::Prompt(_)));
    }

    #[test]
    fn test_complete_prompt_flow() {
        let model = make_test_model();
        let mut events_vec = vec![enter()]; // Enter prompt
        events_vec.extend(type_string("my-passphrase"));
        events_vec.push(enter()); // Submit

        let mut events = ScriptedEventSource::new(events_vec);
        let final_model = simulate(&mut events, model);

        // Should be in generating state (or back to list if no handler)
        assert!(matches!(final_model.screen, Screen::Generating(_)));
    }

    #[test]
    fn test_cancel_prompt_with_esc() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![
            enter(),   // Enter prompt
            char('a'), // Type something
            char('b'),
            esc(), // Cancel
        ]);

        let final_model = simulate(&mut events, model);
        assert!(matches!(final_model.screen, Screen::ArtifactList));
    }
}
