//! Generator module for executing artifact generators and verifying output.
//!
//! This module handles the execution of generator scripts that produce artifact files.
//! Generators are external scripts that create files in a temporary output directory
//! based on user-provided prompts. The module ensures generators produce exactly the
//! expected files and runs them in isolated bubblewrap containers for security.
//!
//! # Generator Scripts
//!
//! Generator scripts are executed in a bubblewrap container with:
//! - `$prompts` directory containing prompt values as files
//! - `$out` directory where generated files should be written
//! - Environment variables: `$out`, `$prompts`, `$artifact_context`, `$machine`/`$username`, `$artifact`
//!
//! # Verification
//!
//! After generation, `verify_generated_files` ensures:
//! - All expected files (defined in artifact.files) are present
//! - No extra files are generated
//! - Files match the expected set exactly
//!
//! # Security
//!
//! Generator execution uses bubblewrap for containerization:
//! - Separate user namespace with uid/gid 1000
//! - Read-only bind mounts for system directories
//! - Minimal writable directories (only $out, $prompts)
//! - Custom /etc/passwd for isolation

use crate::app::model::TargetType;
use crate::backend::helpers::{escape_single_quoted, resolve_path};
use crate::backend::output_capture::{CapturedOutput, run_with_captured_output};
use crate::config::make::ArtifactDef;
use crate::log_debug;
use crate::logging::LogLevel;
use crate::string_vec;
use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;

/// Shared slot for the in-flight bwrap process group id.
///
/// When a generator is running, the background task writes the spawned `nix-shell`
/// child's pid into this slot (which doubles as its pgid because the command is
/// configured with `process_group(0)`). External cancellation logic reads this
/// slot to signal the entire process group with SIGTERM/SIGKILL, killing bwrap
/// and any descendants.
///
/// The slot is `None` when no generator is in flight, `Some(pgid)` while the
/// child is alive, and reset to `None` once the child exits naturally or is
/// killed.
pub type KillSlot = Arc<Mutex<Option<u32>>>;

/// Build a fresh, empty `KillSlot`.
pub fn new_kill_slot() -> KillSlot {
    Arc::new(Mutex::new(None))
}

/// Signal the process group registered in `slot` with SIGTERM, then SIGKILL
/// after a short grace period.
///
/// Returns the pgid that was signalled, or `None` if the slot was empty (no
/// generator in flight). The grace period gives well-behaved scripts a chance
/// to clean up; bwrap's `--unshare-all` containers usually exit on SIGTERM
/// promptly, but a stuck script that ignores SIGTERM is force-killed by the
/// follow-up SIGKILL.
///
/// Shells out to `kill` rather than pulling in `nix` as a dependency — same
/// approach `output_capture.rs` already uses for its timeout escalator.
pub fn signal_kill_slot(slot: &KillSlot) -> Option<u32> {
    let pgid = match slot.lock() {
        Ok(guard) => *guard,
        Err(_) => return None,
    }?;

    // Use the negative pgid form to address the whole process group.
    let neg = format!("-{}", pgid);
    let _ = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(&neg)
        .status();

    // Spawn a delayed SIGKILL on a separate thread so we don't block the
    // background runtime. If the child already exited from SIGTERM, the
    // SIGKILL is a no-op (the pgid no longer exists).
    let neg_kill = neg.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = std::process::Command::new("kill")
            .arg("-KILL")
            .arg(&neg_kill)
            .status();
    });

    Some(pgid)
}

/// Describes the generator invocation context.
///
/// This enum unifies per-target and shared generator execution by abstracting
/// over the differences in how the generator path is obtained and what
/// environment variables are exported.
enum GeneratorContext<'a> {
    /// Per-target generator from an artifact definition.
    PerTarget {
        artifact: &'a ArtifactDef,
        target_type: &'a TargetType,
    },
    /// Shared generator invoked by direct path.
    Shared { generator_path: &'a str },
}

impl<'a> GeneratorContext<'a> {
    /// The raw generator script path string.
    fn generator_script(&self) -> &str {
        match self {
            Self::PerTarget { artifact, .. } => artifact.generator.as_ref(),
            Self::Shared { generator_path } => generator_path,
        }
    }

    /// Build the environment export string for the bwrap command.
    fn env_exports(&self, out: &Path, prompts: &Path, log_level: LogLevel) -> String {
        match self {
            Self::PerTarget {
                artifact,
                target_type,
            } => build_env_exports(out, prompts, target_type, &artifact.name, log_level),
            Self::Shared { .. } => build_shared_env_exports(out, prompts, log_level),
        }
    }

    /// Label for error messages and logging.
    fn label(&self) -> &'static str {
        match self {
            Self::PerTarget { .. } => "generator",
            Self::Shared { .. } => "shared generator",
        }
    }
}

/// Resolve and canonicalize a generator script path.
fn resolve_generator_path(make_base: &Path, generator_script: &str) -> PathBuf {
    let resolved = resolve_path(make_base, generator_script);
    fs::canonicalize(&resolved).unwrap_or(resolved)
}

/// Create a temporary passwd file for bwrap container isolation.
///
/// Returns a `NamedTempFile` that deletes itself on drop, so callers must keep
/// it alive for as long as the path is needed (e.g. for the duration of the
/// bwrap invocation).
fn create_temp_passwd() -> Result<NamedTempFile> {
    let passwd_content = b"user:x:1000:1000::/tmp:/bin/sh\n";
    let mut temp_file = tempfile::Builder::new()
        .prefix("artifacts-cli-passwd-")
        .suffix(".txt")
        .tempfile()
        .context("failed to create temporary passwd file")?;
    temp_file.write_all(passwd_content).with_context(|| {
        format!(
            "failed to write temporary passwd file at {}",
            temp_file.path().display()
        )
    })?;
    Ok(temp_file)
}

/// Build bwrap command arguments for container isolation.
fn build_bwrap_arguments(
    generator_path: &Path,
    prompts: &Path,
    out: &Path,
    temp_passwd: &Path,
) -> Vec<String> {
    let mut args = string_vec!["bwrap"];
    args.extend(string_vec!["--unshare-all", "--unshare-user"]);
    args.extend(string_vec!["--uid", "1000"]);
    args.extend(string_vec!["--gid", "1000"]);
    args.extend(string_vec!["--tmpfs", "/"]);
    args.extend(string_vec!["--chdir", "/"]);
    args.extend(string_vec!["--ro-bind", "/nix/store", "/nix/store"]);
    args.extend(string_vec!["--tmpfs", "/usr/lib/systemd"]);
    args.extend(string_vec!["--proc", "/proc"]);
    args.extend(string_vec!["--dev", "/dev"]);
    args.extend(string_vec!["--bind", prompts.display(), prompts.display()]);
    args.extend(string_vec!["--bind", out.display(), out.display()]);
    if let Some(gen_dir) = generator_path.parent() {
        args.extend(string_vec![
            "--ro-bind",
            gen_dir.display(),
            gen_dir.display()
        ]);
    }
    if Path::new("/bin").exists() {
        args.extend(string_vec!["--ro-bind", "/bin", "/bin"]);
    }
    if Path::new("/usr/bin").exists() {
        args.extend(string_vec!["--ro-bind", "/usr/bin", "/usr/bin"]);
    }
    args.extend(string_vec![
        "--ro-bind",
        temp_passwd.display(),
        "/etc/passwd"
    ]);
    args.extend(string_vec!["--", "/bin/sh"]);
    args.push(generator_path.display().to_string());
    args
}

/// Build environment exports for artifact-specific generators.
fn build_env_exports(
    out: &Path,
    prompts: &Path,
    target_type: &TargetType,
    artifact_name: &str,
    log_level: crate::logging::LogLevel,
) -> String {
    let out_quoted = escape_single_quoted(&out.display().to_string());
    let prompts_quoted = escape_single_quoted(&prompts.display().to_string());
    let artifact_quoted = escape_single_quoted(artifact_name);
    let context_quoted = escape_single_quoted(target_type.context_str());

    match target_type {
        TargetType::HomeManager { username } => {
            let username_quoted = escape_single_quoted(username);
            format!(
                "export out='{}'; export prompts='{}'; export artifact_context='{}'; export username='{}'; export artifact='{}'; export LOG_LEVEL='{}';",
                out_quoted,
                prompts_quoted,
                context_quoted,
                username_quoted,
                artifact_quoted,
                log_level.as_str()
            )
        }
        TargetType::NixOS { machine } => {
            let machine_quoted = escape_single_quoted(machine);
            format!(
                "export out='{}'; export prompts='{}'; export artifact_context='{}'; export machine='{}'; export artifact='{}'; export LOG_LEVEL='{}';",
                out_quoted,
                prompts_quoted,
                context_quoted,
                machine_quoted,
                artifact_quoted,
                log_level.as_str()
            )
        }
    }
}

/// Build environment exports for shared artifact generators.
fn build_shared_env_exports(
    out: &Path,
    prompts: &Path,
    log_level: crate::logging::LogLevel,
) -> String {
    let out_quoted = escape_single_quoted(&out.display().to_string());
    let prompts_quoted = escape_single_quoted(&prompts.display().to_string());
    format!(
        "export out='{}'; export prompts='{}'; export artifact_context='shared'; export LOG_LEVEL='{}';",
        out_quoted,
        prompts_quoted,
        log_level.as_str()
    )
}

/// Execute generator in nix-shell with bwrap containerization.
///
/// The spawned process is placed in its own process group via
/// `process_group(0)` so external cancellation logic can signal the whole tree
/// (bwrap + descendants) with one `kill -<sig> -<pgid>`. When a `kill_slot` is
/// supplied, the child's pid (= pgid) is published into the slot for the
/// duration of the run and cleared when the child returns.
fn execute_generator_in_bwrap(
    nix_shell: &Path,
    arguments: &[String],
    env_exports: &str,
    kill_slot: Option<&KillSlot>,
) -> Result<CapturedOutput> {
    let bwrap_command = arguments.join(" ");
    let run_command = format!("{} {}", env_exports, bwrap_command);

    let mut cmd = std::process::Command::new(nix_shell);
    cmd.arg("-p")
        .arg("bash")
        .arg("bubblewrap")
        .arg("--run")
        .arg(&run_command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // pgid = pid: makes the whole subtree killable as a group.
        .process_group(0);

    let child = cmd
        .spawn()
        .context("failed to start generator in nix-shell")?;
    let child_pid = child.id();

    if let Some(slot) = kill_slot
        && let Ok(mut guard) = slot.lock()
    {
        *guard = Some(child_pid);
    }

    let output = run_with_captured_output(child).context("failed to capture generator output");

    // Clear the slot — child has returned, no further signal can target it
    // safely.
    if let Some(slot) = kill_slot
        && let Ok(mut guard) = slot.lock()
    {
        *guard = None;
    }

    output
}

/// Log the bwrap command with formatted output.
#[cfg(feature = "logging")]
fn log_bwrap_command(arguments: &[String]) {
    let mut result = String::new();
    for (index, argument) in arguments.iter().enumerate() {
        if index == 0 {
            result.push_str(argument);
        } else if argument.starts_with("--") {
            result.push_str(" \\\n");
            result.push_str(argument);
        } else {
            result.push(' ');
            result.push_str(argument);
        }
    }
    log_debug!("{}", result);
}

/// Log the bwrap command (no-op when logging is disabled).
#[cfg(not(feature = "logging"))]
fn log_bwrap_command(_arguments: &[String]) {}

/// Internal unified implementation for running generator scripts.
///
/// This function contains the common logic shared by both per-target and shared
/// generator execution paths. When `kill_slot` is `Some`, the spawned child's
/// pgid is published there so external cancellation can signal the process
/// group.
fn run_generator_inner(
    ctx: &GeneratorContext<'_>,
    make_base: &Path,
    prompts: &Path,
    out: &Path,
    log_level: LogLevel,
    kill_slot: Option<&KillSlot>,
) -> Result<CapturedOutput> {
    let generator_path = resolve_generator_path(make_base, ctx.generator_script());
    let nix_shell = which::which("nix-shell")
        .context("nix-shell is required to run the generator but was not found in PATH")?;
    let temp_passwd = create_temp_passwd()?;
    let arguments = build_bwrap_arguments(&generator_path, prompts, out, temp_passwd.path());

    log_debug!(
        "run {} bwrap with command {}",
        ctx.label(),
        generator_path.display()
    );
    log_bwrap_command(&arguments);

    let env_exports = ctx.env_exports(out, prompts, log_level);
    let output = execute_generator_in_bwrap(&nix_shell, &arguments, &env_exports, kill_slot)?;

    // `temp_passwd` is deleted automatically when it goes out of scope.

    if !output.exit_success {
        bail!(
            "{} failed inside nix-shell with non-zero exit status",
            ctx.label()
        );
    }

    Ok(output)
}

/// Verify that the generator produced exactly the expected files for the given artifact.
///
/// This function checks that the generator script produced:
/// - All files defined in `artifact.files` (no missing files)
/// - No additional files not defined in `artifact.files` (no extra files)
///
/// # Arguments
///
/// * `artifact` - The artifact definition containing the expected file set
/// * `out_path` - Path to the directory where generator output is located
///
/// # Returns
///
/// Returns `Ok(())` if the file set matches exactly.
///
/// # Errors
///
/// Returns an error if:
/// - The output directory cannot be read
/// - Required files are missing from the output
/// - Extra files were generated that aren't in the artifact definition
pub fn verify_generated_files(artifact: &ArtifactDef, out_path: &Path) -> Result<()> {
    let expected_files: HashSet<String> = artifact.files.keys().cloned().collect();

    let actual_files: HashSet<String> = fs::read_dir(out_path)
        .with_context(|| format!("reading generator output dir {}", out_path.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();

    let mut missing_files: Vec<String> =
        expected_files.difference(&actual_files).cloned().collect();

    if !missing_files.is_empty() {
        missing_files.sort();
        return Err(anyhow::anyhow!(
            "generator missing required files for artifact '{}': [{}]",
            artifact.name,
            missing_files.join(", ")
        ));
    }

    let mut unwanted_files: Vec<String> =
        actual_files.difference(&expected_files).cloned().collect();

    if !unwanted_files.is_empty() {
        unwanted_files.sort();
        return Err(anyhow::anyhow!(
            "generator produced extra files for artifact '{}': [{}]",
            artifact.name,
            unwanted_files.join(", ")
        ));
    }

    Ok(())
}

/// Run a generator script for an artifact in an isolated bubblewrap container.
///
/// This function executes the artifact's generator script inside a nix-shell environment
/// with bubblewrap containerization for security. The generator script is expected to
/// create files in the `$out` directory based on values from the `$prompts` directory.
///
/// # Environment Variables
///
/// The generator script receives these environment variables:
/// * `$out` - Path to the output directory where files should be created
/// * `$prompts` - Path to directory containing prompt values as files
/// * `$artifact_context` - Context string: "nixos", "homemanager", or "shared"
/// * `$machine` - Machine name (for nixos context) or `$username` (for homemanager context)
/// * `$artifact` - Name of the artifact being generated
/// * `$LOG_LEVEL` - Log level for script verbosity ("debug", "info", "warn", "error")
///
/// # Arguments
///
/// * `artifact` - The artifact definition containing the generator script path
/// * `target_type` - Target type with name (Nixos { machine }, HomeManager { username }, or Shared)
/// * `make_base` - Base path for resolving relative script paths
/// * `prompts` - Directory containing prompt values as files
/// * `out` - Directory where generator should create output files
/// * `log_level` - Log level to pass to the script
///
/// # Returns
///
/// Returns the captured output from the generator script execution.
///
/// # Errors
///
/// Returns an error if:
/// - nix-shell is not found in PATH
/// - The generator script cannot be found or executed
/// - The generator script exits with non-zero status
pub fn run_generator_script(
    artifact: &ArtifactDef,
    target_type: &TargetType,
    make_base: &Path,
    prompts: &Path,
    out: &Path,
    log_level: LogLevel,
    kill_slot: Option<&KillSlot>,
) -> Result<CapturedOutput> {
    let ctx = GeneratorContext::PerTarget {
        artifact,
        target_type,
    };
    run_generator_inner(&ctx, make_base, prompts, out, log_level, kill_slot)
}

/// Run a generator script by its direct path (for shared artifacts).
///
/// This is a simplified version of `run_generator_script` that doesn't require
/// a full artifact definition. It's used for shared artifacts where the generator
/// path is known directly rather than extracted from an artifact definition.
///
/// The generator receives a minimal environment:
/// * `$out` - Path to the output directory where files should be created
/// * `$prompts` - Path to directory containing prompt values as files
/// * `$artifact_context` - Always set to "shared" for this function
/// * `$LOG_LEVEL` - Log level for script verbosity ("debug", "info", "warn", "error")
///
/// # Arguments
///
/// * `generator_path` - Path to the generator script to execute
/// * `make_base` - Base path for resolving relative script paths
/// * `prompts` - Directory containing prompt values as files
/// * `out` - Directory where generator should create output files
/// * `log_level` - Log level to pass to the script
///
/// # Returns
///
/// Returns the captured output from the generator script execution.
///
/// # Errors
///
/// Returns an error if:
/// - nix-shell is not found in PATH
/// - The generator script cannot be found or executed
/// - The generator script exits with non-zero status
///
/// # Example
///
/// ```rust,ignore
/// let output = run_generator_script_with_path(
///     "./generators/ssh_key.sh",
///     Path::new("/project"),
///     Path::new("/tmp/prompts"),
///     Path::new("/tmp/out"),
///     LogLevel::Debug,
/// )?;
/// assert!(output.exit_success);
/// ```
pub fn run_generator_script_with_path(
    generator_path: &str,
    make_base: &Path,
    prompts: &Path,
    out: &Path,
    log_level: LogLevel,
    kill_slot: Option<&KillSlot>,
) -> Result<CapturedOutput> {
    let ctx = GeneratorContext::Shared { generator_path };
    run_generator_inner(&ctx, make_base, prompts, out, log_level, kill_slot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::{Duration, Instant};

    #[test]
    fn signal_kill_slot_returns_none_when_empty() {
        let slot = new_kill_slot();
        assert!(signal_kill_slot(&slot).is_none());
    }

    #[test]
    fn signal_kill_slot_terminates_process_group_within_grace() {
        // End-to-end exercise of the cancel path without bwrap: spawn a long
        // sleep in its own process group, publish its pgid to a KillSlot, ask
        // signal_kill_slot to do the SIGTERM/SIGKILL dance, and assert the
        // child is reaped within the grace window. If this test ever flakes
        // by timing out, that's a regression in the kill plumbing — the
        // sleep itself is 30s, so a 3s wait gives the kill mechanism plenty
        // of headroom over its 500ms SIGTERM→SIGKILL cadence.
        let mut child = Command::new("sh")
            .arg("-c")
            .arg("sleep 30")
            .process_group(0)
            .spawn()
            .expect("spawn sleep child");
        let pid = child.id();

        let slot = new_kill_slot();
        *slot.lock().unwrap() = Some(pid);

        let signalled = signal_kill_slot(&slot).expect("slot held a pid");
        assert_eq!(signalled, pid);

        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(_status) = child.try_wait().expect("try_wait") {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("child was not killed within grace window");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
