use artifacts_cli::app::model::*;
use artifacts_cli::config::make::{ArtifactDef, FileDef, PromptDef};
use artifacts_cli::tui::views::{render_artifact_list, render_progress, render_prompt};
use insta::assert_snapshot;
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use std::collections::BTreeMap;

/// Convert a ratatui Buffer to a string for snapshot testing.
/// This produces a human-readable representation of the terminal output.
fn buffer_to_string(buf: &Buffer) -> String {
    let mut output = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            output.push_str(cell.symbol());
        }
        output.push('\n');
    }
    // Trim trailing whitespace from each line for cleaner snapshots
    output
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

fn make_test_artifact(name: &str, prompts: Vec<&str>) -> ArtifactDef {
    let mut prompt_map = BTreeMap::new();
    for p in prompts {
        prompt_map.insert(
            p.to_string(),
            PromptDef {
                name: p.to_string(),
                description: Some(format!("Enter the {} value", p)),
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
                path: Some("/test/path".to_string()),
                owner: None,
                group: None,
            },
        )]),
        prompts: prompt_map,
        generator: "/nix/store/xxx-gen".to_string(),
        serialization: "test-backend".to_string(),
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
                status: ArtifactStatus::UpToDate,
            },
            ArtifactEntry {
                target: "user@host".to_string(),
                target_type: TargetType::HomeManager,
                artifact: make_test_artifact("gpg-key", vec!["email", "name"]),
                status: ArtifactStatus::NeedsGeneration,
            },
        ],
        selected_index: 0,
        error: None,
    }
}

// ============================================================================
// Artifact List View Tests
// ============================================================================

#[test]
fn test_artifact_list_initial() {
    let model = make_test_model();

    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

#[test]
fn test_artifact_list_with_selection() {
    let mut model = make_test_model();
    model.selected_index = 1;

    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

#[test]
fn test_artifact_list_with_failed_status() {
    let mut model = make_test_model();
    model.artifacts[0].status =
        ArtifactStatus::Failed("Generator script exited with code 1".to_string());

    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_artifact_list(f, &model, f.area()))
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

// ============================================================================
// Prompt View Tests
// ============================================================================

#[test]
fn test_prompt_initial_line_mode() {
    let state = PromptState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        prompts: vec![PromptEntry {
            name: "passphrase".to_string(),
            description: Some("Enter the SSH key passphrase".to_string()),
        }],
        current_prompt_index: 0,
        input_mode: InputMode::Line,
        buffer: String::new(),
        collected: Default::default(),
    };

    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_prompt(f, &state, f.area()))
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

#[test]
fn test_prompt_with_input() {
    let state = PromptState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        prompts: vec![PromptEntry {
            name: "passphrase".to_string(),
            description: Some("Enter the SSH key passphrase".to_string()),
        }],
        current_prompt_index: 0,
        input_mode: InputMode::Line,
        buffer: "my-secret-pass".to_string(),
        collected: Default::default(),
    };

    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_prompt(f, &state, f.area()))
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

#[test]
fn test_prompt_hidden_mode() {
    let state = PromptState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        prompts: vec![PromptEntry {
            name: "passphrase".to_string(),
            description: Some("Enter the SSH key passphrase".to_string()),
        }],
        current_prompt_index: 0,
        input_mode: InputMode::Hidden,
        buffer: "secret123".to_string(),
        collected: Default::default(),
    };

    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_prompt(f, &state, f.area()))
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

#[test]
fn test_prompt_multiline_mode() {
    let state = PromptState {
        artifact_index: 0,
        artifact_name: "certificate".to_string(),
        prompts: vec![PromptEntry {
            name: "pem".to_string(),
            description: Some("Paste the certificate PEM content".to_string()),
        }],
        current_prompt_index: 0,
        input_mode: InputMode::Multiline,
        buffer: "-----BEGIN CERT-----".to_string(),
        collected: Default::default(),
    };

    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_prompt(f, &state, f.area()))
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

#[test]
fn test_prompt_second_of_three() {
    let mut collected = std::collections::HashMap::new();
    collected.insert("email".to_string(), "test@example.com".to_string());

    let state = PromptState {
        artifact_index: 0,
        artifact_name: "gpg-key".to_string(),
        prompts: vec![
            PromptEntry {
                name: "email".to_string(),
                description: Some("Enter email address".to_string()),
            },
            PromptEntry {
                name: "name".to_string(),
                description: Some("Enter full name".to_string()),
            },
            PromptEntry {
                name: "passphrase".to_string(),
                description: Some("Enter GPG passphrase".to_string()),
            },
        ],
        current_prompt_index: 1,
        input_mode: InputMode::Line,
        buffer: String::new(),
        collected,
    };

    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_prompt(f, &state, f.area()))
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

// ============================================================================
// Progress View Tests
// ============================================================================

#[test]
fn test_progress_running_generator() {
    let state = GeneratingState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        step: GenerationStep::RunningGenerator,
        log_lines: vec![],
    };

    let backend = TestBackend::new(60, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_progress(f, &state, f.area()))
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}

#[test]
fn test_progress_serializing() {
    let state = GeneratingState {
        artifact_index: 0,
        artifact_name: "ssh-key".to_string(),
        step: GenerationStep::Serializing,
        log_lines: vec![
            "Generator completed successfully".to_string(),
            "Starting serialization...".to_string(),
        ],
    };

    let backend = TestBackend::new(60, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| render_progress(f, &state, f.area()))
        .unwrap();

    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}
