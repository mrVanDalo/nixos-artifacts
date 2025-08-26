use crate::backend::resolve_path;
use crate::config::make::ArtifactDef;
use std::fs;
use std::path::{Path, PathBuf};

pub struct GeneratorScript {}

// A thin wrapper around the existing run_generator function, similar in spirit to PromptManager
pub struct GeneratorManger {}

impl GeneratorManger {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run_generator_script(
        &self,
        artifact: &ArtifactDef,
        make_base: &Path,
        prompts: &Path,
        out: &Path,
    ) {
        let generator_script = artifact.generator.as_ref();
        let generator_script_path = resolve_path(make_base, generator_script);
        let generator_script_absolut_path = fs::canonicalize(&generator_script_path)
            .unwrap_or_else(|_| generator_script_path.clone());

        // Only use nix-shell. If it fails, crash the program.
        let nix_shell = which::which("nix-shell")
            .expect("nix-shell is required to run the generator but was not found in PATH");

        // Build the bwrap command as a single string for nix-shell --run
        let mut arguments: Vec<String> = Vec::new();
        arguments.push("bwrap".to_string());
        arguments.push("--ro-bind".to_string());
        arguments.push("/nix/store".to_string());
        arguments.push("/nix/store".to_string());
        arguments.push("--tmpfs".to_string());
        arguments.push("/usr/lib/systemd".to_string());
        arguments.push("--dev".to_string());
        arguments.push("/dev".to_string());
        arguments.push("--bind".to_string());
        arguments.push(prompts.display().to_string());
        arguments.push(prompts.display().to_string());
        arguments.push("--bind".to_string());
        arguments.push(out.display().to_string());
        arguments.push(out.display().to_string());
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
        match generator_command.status() {
            Ok(status) => {
                if !status.success() {
                    panic!(
                        "generator failed inside nix-shell with exit status: {}",
                        status
                    );
                }
            }
            Err(e) => {
                panic!("failed to start generator in nix-shell: {}", e);
            }
        }
    }
}
