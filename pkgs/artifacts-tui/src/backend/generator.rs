use crate::backend::resolve_path;
use crate::config::make::ArtifactDef;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;

// A thin wrapper around the existing run_generator function, similar in spirit to PromptManager
pub struct GeneratorManger {}

impl GeneratorManger {
    pub fn new() -> Self {
        Self {}
    }

    // todo: get rid of nix-shell and use bwrap directly (brwap must be part of the nix-package)
    pub fn run_generator_script(
        &self,
        artifact: &ArtifactDef,
        make_base: &Path,
        prompts: &Path,
        out: &Path,
    ) -> Result<()> {
        let generator_script = artifact.generator.as_ref();
        let generator_script_path = resolve_path(make_base, generator_script);
        let generator_script_absolut_path = fs::canonicalize(&generator_script_path)
            .unwrap_or_else(|_| generator_script_path.clone());

        // Only use nix-shell. If it fails, return an error to stop the program gracefully.
        let nix_shell = which::which("nix-shell")
            .context("nix-shell is required to run the generator but was not found in PATH")?;

        // Build the bwrap command as a single string for nix-shell --run
        // Start with the always-present arguments using vec![] to appease clippy
        let mut arguments: Vec<String> = vec![
            "bwrap".to_string(),
            "--ro-bind".to_string(),
            "/nix/store".to_string(),
            "/nix/store".to_string(),
            "--tmpfs".to_string(),
            "/usr/lib/systemd".to_string(),
            "--dev".to_string(),
            "/dev".to_string(),
            "--bind".to_string(),
            prompts.display().to_string(),
            prompts.display().to_string(),
            "--bind".to_string(),
            out.display().to_string(),
            out.display().to_string(),
        ];
        if let Some(gen_dir) = generator_script_absolut_path.parent() {
            arguments.push("--ro-bind".to_string());
            arguments.push(gen_dir.display().to_string());
            arguments.push(gen_dir.display().to_string());
        }
        if Path::new("/bin").exists() {
            arguments.push("--ro-bind".to_string());
            arguments.push("/bin".to_string());
            arguments.push("/bin".to_string());
        }
        if Path::new("/usr/bin").exists() {
            arguments.push("--ro-bind".to_string());
            arguments.push("/usr/bin".to_string());
            arguments.push("/usr/bin".to_string());
        }
        arguments.push("--unshare-all".to_string());
        arguments.push("--unshare-user".to_string());
        arguments.push("--uid".to_string());
        arguments.push("1000".to_string());
        arguments.push("--".to_string());
        arguments.push("/bin/sh".to_string());
        arguments.push(generator_script_absolut_path.display().to_string());
        let bwrap_command = arguments.join(" ");

        // Ensure that our 'out' and 'prompt' override any nix-shell provided 'out'
        fn sh_escape_single_quoted(s: &str) -> String {
            // Replace ' with '\'' for safe single-quoting
            s.replace('\'', "'\\''")
        }
        let out_quoted = sh_escape_single_quoted(&out.display().to_string());
        let prompt_quoted = sh_escape_single_quoted(&prompts.display().to_string());
        let nix_shell_run_command = format!(
            "export out='{}'; export prompt='{}'; {}",
            out_quoted, prompt_quoted, bwrap_command
        );

        let mut generator_command = std::process::Command::new(nix_shell);
        generator_command
            .arg("-p")
            .arg("bash")
            .arg("bubblewrap")
            .arg("--run")
            .arg(&nix_shell_run_command);

        // Do not pass 'out' or 'prompt' here to avoid being overridden by nix-shell internals
        let status = generator_command
            .status()
            .context("failed to start generator in nix-shell")?;

        if !status.success() {
            bail!(
                "generator failed inside nix-shell with exit status: {}",
                status
            );
        }

        Ok(())
    }
}
