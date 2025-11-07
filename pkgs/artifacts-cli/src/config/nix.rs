use crate::backend::helpers::pretty_print_shell_escape;
use log::{debug, trace};

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

    // Pretty-print: shell-escaped command line
    let prog = nix_bin.to_string_lossy().to_string();
    let pretty = std::iter::once(pretty_print_shell_escape(&prog))
        .chain(
            arguments
                .iter()
                .map(|argument| pretty_print_shell_escape(argument)),
        )
        .collect::<Vec<_>>()
        .join(" ");
    debug!("Running nix build on {}", flake_path.display());
    trace!("{}", pretty);

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
