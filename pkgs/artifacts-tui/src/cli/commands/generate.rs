use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

/// Stub implementation of the generate workflow per guidelines.
/// This creates temporary prompt and out directories, logs the intended steps,
/// and cleans up.
pub fn run(backend_toml: &Path, make_json: &Path) -> Result<()> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let base = std::env::temp_dir().join(format!("artifacts-tui-{}", now));
    let prompts = base.join("prompts");
    let out = base.join("out");

    fs::create_dir_all(&prompts).context("creating prompts directory")?;
    fs::create_dir_all(&out).context("creating out directory")?;

    // In the future, iterate artifacts from make.json and backend.toml
    println!("[generate] backend: {}", backend_toml.display());
    println!("[generate] make: {}", make_json.display());
    println!("[generate] prompts dir: {}", prompts.display());
    println!("[generate] out dir: {}", out.display());
    println!("[generate] TODO: prompt user for inputs and write to prompts/*");
    println!("[generate] TODO: execute generator script in bubblewrap with $prompts and $out");
    println!(
        "[generate] TODO: verify generated files, then serialize via backend serialize script"
    );

    // For now, just verify directories exist and then clean up
    if !prompts.is_dir() || !out.is_dir() {
        bail!("failed to prepare temporary directories");
    }

    // Cleanup
    fs::remove_dir_all(&base).with_context(|| format!("removing temp base {}", base.display()))?;

    Ok(())
}
