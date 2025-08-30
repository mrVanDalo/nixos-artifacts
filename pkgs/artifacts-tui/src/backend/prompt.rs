use crate::config::make::{ArtifactDef, PromptDef};
use anyhow::{Context, Result};
use crossterm::{
    QueueableCommand, cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    style::Print,
    terminal,
};
use std::collections::HashMap;
use std::io::{BufRead, IsTerminal, Stdin, Write};
use std::path::Path;
use std::{fs, io};

#[derive(Debug)]
pub struct PromptResult {
    pub results: HashMap<String, String>,
}

impl PromptResult {
    pub fn write_prompts_to_files(&self, dir: &Path) -> Result<()> {
        for (name, value) in &self.results {
            let file_path = dir.join(name);
            fs::write(&file_path, value)
                .with_context(|| format!("failed to write prompt file {}", file_path.display()))?;
        }
        Ok(())
    }
}

pub fn read_artifact_prompts(artifact: &ArtifactDef) -> Result<PromptResult> {
    let mut results = HashMap::new();

    if artifact.prompts.is_empty() {
        return Ok(PromptResult { results });
    }

    for prompt_element in artifact.prompts.values() {
        let (name, value) = read_prompt(prompt_element)?;
        results.insert(name, value.clone());
    }

    Ok(PromptResult { results })
}

fn read_prompt(prompt_element: &PromptDef) -> Result<(String, String)> {
    let description = if let Some(desc) = &prompt_element.description {
        desc
    } else {
        "no description given"
    };

    let stdin = io::stdin();
    let value = if stdin.is_terminal() {
        interactive_read_prompt(&prompt_element.name, description)?
    } else {
        non_interactive_read_prompt(prompt_element, description, stdin)?
    };

    Ok((prompt_element.name.clone(), value))
}

fn non_interactive_read_prompt(
    prompt_element: &PromptDef,
    description: &str,
    stdin: Stdin,
) -> Result<String> {
    println!(">>> DESC: {}", description);
    println!(">>> PROMPT: {}", prompt_element.name);
    println!("> ");
    io::stdout().flush()?;
    let mut reader = stdin.lock();
    let mut input = String::new();
    reader
        .read_line(&mut input)
        .context("Error reading piped input")?;
    Ok(input)
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum InputMode {
    Line,
    Multiline,
    Hidden,
}

// Helper: re-render only the current prompt line (no full-screen clear)
fn render_prompt_line(
    stdout: &mut io::Stdout,
    mode: InputMode,
    buffer: &str,
    show_hint: bool,
) -> Result<usize> {
    let mode_str = match mode {
        InputMode::Line => "line",
        InputMode::Multiline => "multiline",
        InputMode::Hidden => "hidden",
    };
    stdout.queue(cursor::MoveToColumn(0))?; // go to column 0
    stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine))?; // clear the current line
    let prompt_prefix = if show_hint {
        format!("[{}] > (Tab to change mode): ", mode_str)
    } else {
        format!("[{}] > : ", mode_str)
    };
    let prompt_len = prompt_prefix.chars().count();
    stdout.queue(Print(prompt_prefix))?;
    if mode == InputMode::Hidden {
        stdout.queue(Print("*".repeat(buffer.chars().count())))?;
    } else {
        stdout.queue(Print(buffer))?;
    }
    stdout.flush()?;
    Ok(prompt_len)
}

fn interactive_read_prompt(prompt_name: &str, description: &str) -> Result<String> {
    let mut stdout = io::stdout();

    println!(">>> DESC: {}", description);
    println!(">>> PROMPT: {}", prompt_name);

    // Enable raw mode for key handling
    terminal::enable_raw_mode()?;

    let mut mode = InputMode::Line;
    let mut buffer = String::new();

    // Initial prompt render (show hint before typing starts)
    let mut typing_started = false;
    let mut prompt_prefix_len = render_prompt_line(&mut stdout, mode, &buffer, true)?;

    loop {
        if event::poll(std::time::Duration::from_millis(500))? {
            if let Event::Key(key) = event::read()? {
                // Only handle key presses (not repeats/releases) where applicable
                if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                    match (key.code, key.modifiers) {
                        // Tab: switch to next mode
                        (KeyCode::Tab, KeyModifiers::NONE) => {
                            // Only allow changing mode before typing anything
                            if buffer.is_empty() {
                                mode = match mode {
                                    InputMode::Line => InputMode::Multiline,
                                    InputMode::Multiline => InputMode::Hidden,
                                    InputMode::Hidden => InputMode::Line,
                                };
                                prompt_prefix_len =
                                    render_prompt_line(&mut stdout, mode, &buffer, true)?;
                            }
                        }

                        // Enter: submit input or new multiline line
                        (KeyCode::Enter, _) => match mode {
                            InputMode::Line | InputMode::Hidden => {
                                buffer.push('\n');
                                terminal::disable_raw_mode()?;
                                println!();
                                return Ok(buffer);
                            }
                            InputMode::Multiline => {
                                buffer.push('\n');
                                stdout.queue(Print("\r\n"))?;
                                stdout.flush()?;
                            }
                        },

                        // Ctrl-D: submit input or new multiline line
                        (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                            if mode == InputMode::Multiline {
                                terminal::disable_raw_mode()?;
                                println!();
                                return Ok(buffer);
                            }
                        }

                        // Esc or Ctrl-C: interrupt and stop the program
                        (KeyCode::Char('c'), KeyModifiers::CONTROL)
                        | (KeyCode::Char('C'), KeyModifiers::CONTROL)
                        | (KeyCode::Esc, _) => {
                            // restore terminal and propagate an interruption error
                            stdout.queue(Print("\r\n"))?;
                            stdout.flush()?;
                            terminal::disable_raw_mode()?;
                            return Err(anyhow::anyhow!("Interrupted"));
                        }

                        // Regular character input
                        (KeyCode::Char(c), mods) => {
                            if !mods.is_empty() && mods != KeyModifiers::SHIFT {
                                return Ok(buffer);
                            }

                            if !typing_started && buffer.is_empty() {
                                // First typed character: switch to no-hint rendering
                                typing_started = true;
                                buffer.push(c);
                                // We re-rendered the whole line including the char, so skip incremental echoing
                                prompt_prefix_len =
                                    render_prompt_line(&mut stdout, mode, &buffer, false)?;
                            } else {
                                buffer.push(c);
                                match mode {
                                    InputMode::Hidden => {
                                        stdout.queue(Print('*'))?;
                                        stdout.flush()?;
                                    }
                                    InputMode::Line => {
                                        stdout.queue(Print(c))?;
                                        stdout.flush()?;
                                    }
                                    InputMode::Multiline => {
                                        stdout.queue(Print(c))?;
                                        stdout.flush()?;
                                    }
                                }
                            }
                        }

                        // Backspace: delete last character
                        (KeyCode::Backspace, _) => {
                            if let Some(c) = buffer.pop() {
                                // Handle visual deletion on the terminal for single-line and multiline cases
                                if c != '\n' {
                                    stdout.queue(cursor::MoveLeft(1))?;
                                    stdout.queue(Print(' '))?;
                                    stdout.queue(cursor::MoveLeft(1))?;
                                    stdout.flush()?;
                                } else if mode == InputMode::Multiline {
                                    // Move cursor to end of previous line to reflect removed newline
                                    // Determine length of the previous line segment in the remaining buffer
                                    let last_line_len = match buffer.rsplit_once('\n') {
                                        Some((_, tail)) => tail.chars().count(),
                                        None => buffer.chars().count(),
                                    };
                                    // If there are still newlines, we're moving to a non-first input line (column starts at 0).
                                    // If no newlines remain, we're moving back to the first input line, which begins after the prompt prefix.
                                    let base_col = if buffer.contains('\n') {
                                        0
                                    } else {
                                        prompt_prefix_len
                                    };
                                    let target_col = base_col + last_line_len;
                                    stdout.queue(cursor::MoveUp(1))?;
                                    stdout.queue(cursor::MoveToColumn(target_col as u16))?;
                                    stdout.flush()?;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
