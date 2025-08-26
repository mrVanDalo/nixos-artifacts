use crate::config::make::{ArtifactDef, PromptDef};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::{BufRead, IsTerminal};
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

pub struct PromptManager {}

impl PromptManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn query_prompts(&self, artifact: &ArtifactDef) -> Result<PromptResult> {
        let mut results = HashMap::new();

        if artifact.prompts.is_empty() {
            return Ok(PromptResult { results });
        }

        for prompt_element in &artifact.prompts {
            let (name, value) = self.read_prompt(prompt_element)?;
            results.insert(name, value.clone());
        }

        Ok(PromptResult { results })
    }

    fn read_prompt(&self, prompt_element: &PromptDef) -> Result<(String, String)> {
        let description = prompt_element
            .description
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No description provided for prompt"))?;

        println!("{}", description);
        println!("enter prompt {}: ", prompt_element.name);

        let stdin = io::stdin();
        let value = if stdin.is_terminal() {
            // Interactive mode - read line directly
            let mut input = String::new();
            stdin
                .read_line(&mut input)
                .context("Error reading interactive input")?;
            input
        } else {
            // Non-interactive mode - use buffered reading
            let mut reader = stdin.lock();
            let mut input = String::new();
            reader
                .read_line(&mut input)
                .context("Error reading non-interactive input")?;
            input
        };

        Ok((prompt_element.name.clone(), value))
    }
}
