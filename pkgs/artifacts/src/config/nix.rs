//! Nix expression building and evaluation.
//!
//! This module handles the execution of `nix build` to extract artifact
//! configurations from a flake.nix file. It builds and runs a Nix expression
//! that traverses `nixosConfigurations` and `homeConfigurations` to produce
//! a JSON file containing all artifact definitions.
//!
//! ## Nix Expression
//!
//! The [`build_make_from_flake`] function uses an embedded Nix expression
//! (from `make_expr.nix`) that:
//!
//! 1. Imports the flake at the given path
//! 2. Traverses `nixosConfigurations.<machine>.config.artifacts.store`
//! 3. Traverses `homeConfigurations."<user>@<host>".config.artifacts.store`
//! 4. Collects artifact definitions into a structured format
//! 5. Writes the result as JSON to the Nix store
//!
//! ## Usage Flow
//!
//! 1. Call [`build_make_from_flake`] with the path to flake.nix
//! 2. Nix builds the expression and outputs a store path
//! 3. The store path points to a JSON file (make.json)
//! 4. Pass the path to [`MakeConfiguration::read_make_config`](super::make::MakeConfiguration)
//!
//! ## Error Handling
//!
//! Returns errors if:
//! - Nix is not installed or not in PATH
//! - The flake.nix cannot be evaluated
//! - The Nix expression fails to build
//! - The output path is not a valid file

use crate::backend::helpers::pretty_print_shell_escape;
use crate::log_debug;

/// Build the make.json file from a flake.nix by running `nix build`.
///
/// This function executes a Nix build that extracts artifact configurations
/// from the flake's `nixosConfigurations` and `homeConfigurations`. The
/// result is a JSON file containing all artifact definitions, which can then
/// be parsed using [`MakeConfiguration::read_make_config`](super::make::MakeConfiguration).
///
/// ## Arguments
///
/// * `flake_path` - Path to the directory containing flake.nix
///
/// ## Returns
///
/// The path to the generated make.json file in the Nix store.
///
/// ## Errors
///
/// Returns an error if:
/// - The `nix` command is not found in PATH
/// - The Nix build fails (invalid flake, evaluation error)
/// - The build succeeds but returns an empty or invalid path
///
/// ## Example
///
/// ```rust,ignore
/// use std::path::Path;
///
/// let make_json_path = build_make_from_flake(Path::new("."))?;
/// let config = MakeConfiguration::read_make_config(&make_json_path)?;
/// ```
pub fn build_make_from_flake(flake_path: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
    // Ensure nix is available
    let nix_bin = which::which("nix")
        .map_err(|_| anyhow::anyhow!("'nix' command not found in PATH. Please install Nix."))?;
    let mut command = std::process::Command::new(&nix_bin);
    let expr = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/config/make_expr.nix"
    ));
    // Prepare args using string_vec! so flags and their values stay adjacent in logs
    let mut arguments: Vec<String> = string_vec!["build"];
    arguments.extend(string_vec!["--impure"]);
    arguments.extend(string_vec!["-I", format!("flake={}", flake_path.display())]);
    arguments.extend(string_vec!["--no-link"]);
    arguments.extend(string_vec!["--print-out-paths"]);
    arguments.extend(string_vec!["--expr", expr]);

    // Attach all arguments at once
    let prog = nix_bin.to_string_lossy().to_string();
    let _pretty = std::iter::once(pretty_print_shell_escape(&prog))
        .chain(
            arguments
                .iter()
                .map(|argument| pretty_print_shell_escape(argument)),
        )
        .collect::<Vec<_>>()
        .join(" ");
    log_debug!("Running nix build on {}", flake_path.display());
    log_debug!("{}", _pretty);

    command.args(&arguments);
    let output = command
        .output()
        .map_err(|e| anyhow::anyhow!("failed to start nix build: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow::anyhow!(
            "nix build failed. stdout: {}\nstderr: {}",
            stdout,
            stderr
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let path_line = stdout.lines().last().unwrap_or("").trim();
    if path_line.is_empty() {
        return Err(anyhow::anyhow!("nix build did not return a store path"));
    }
    let make_path = std::path::Path::new(path_line).to_path_buf();
    if !make_path.is_file() {
        return Err(anyhow::anyhow!(
            "nix build returned a path that is not a file: {}",
            make_path.display()
        ));
    }
    Ok(make_path)
}
