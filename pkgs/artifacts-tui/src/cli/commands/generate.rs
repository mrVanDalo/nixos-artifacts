use crate::backend::generator::{run_generator_script, verify_generated_files};
use crate::backend::helpers::{print_files, resolve_path};
use crate::backend::prompt::read_artifact_prompts;
use crate::backend::serialization::run_serialize;
use crate::backend::temp_dir::create_temp_dir;
use crate::config::backend::BackendConfiguration;
use crate::config::make::{ArtifactDef, MakeConfiguration};
use anyhow::{Context, Result, bail};
use log::{debug, info};
use serde_json::{json, to_string_pretty};
use std::fs;
use std::path::Path;

pub fn run_generate_command(
    backend_toml: &Path,
    make_json: &Path,
    all: bool,
    machines_to_regenerate: &Vec<String>,
    artifacts_to_regenerate: &Vec<String>,
) -> Result<()> {
    // Validate argument rules
    if all {
        if !machines_to_regenerate.is_empty() || !artifacts_to_regenerate.is_empty() {
            bail!("--all conflicts with --machine/--artifact");
        }
    }

    let all = if !all {
        machines_to_regenerate.is_empty() && artifacts_to_regenerate.is_empty()
    } else {
        all
    };

    run_generate_workflow(
        backend_toml,
        make_json,
        all,
        machines_to_regenerate,
        artifacts_to_regenerate,
        true,
        "[generate]",
    )
}

pub fn run_generate_workflow(
    backend_toml: &Path,
    make_json: &Path,
    all: bool,
    machines_to_regenerate: &Vec<String>,
    artifacts_to_regenerate: &Vec<String>,
    check_if_artifact_exists: bool,
    header: &str,
) -> Result<()> {
    let backend = BackendConfiguration::read_backend_config(backend_toml)?;
    let make = MakeConfiguration::read_make_config(make_json)?;

    let check_all = || -> bool { all };

    let check_machine = |machine: &str| -> bool {
        machines_to_regenerate.is_empty() || machines_to_regenerate.iter().any(|m| m == machine)
    };

    let check_artifact = |artifact: &ArtifactDef| -> bool {
        artifacts_to_regenerate.is_empty()
            || artifacts_to_regenerate.iter().any(|a| a == &artifact.name)
    };

    for (machine, artifacts) in &make.make_map {
        for artifact in artifacts.values() {
            if !(check_all() || check_machine(machine) && check_artifact(&artifact)) {
                println!("✅ {}/{}", machine, artifact.name);
                continue;
            }

            info!("{}", header);

            print_files(&artifact, &make.make_base);

            if check_if_artifact_exists {
                let skip_rest =
                    run_check_serialization(&artifact, &machine, &backend, &make.make_base)?;
                if skip_rest {
                    println!("✅ {}/{}", machine, artifact.name);
                    continue;
                }
            }

            println!("⚡ {}/{}", machine, artifact.name);
            let prompt_results =
                read_artifact_prompts(&artifact).context("could not query all prompts")?;

            let prompt = create_temp_dir(Some(&format!("prompt-{}", artifact.name)))?;
            prompt_results
                .write_prompts_to_files(&prompt.path_buf)
                .context("writing prompts to files")?;

            let out = create_temp_dir(Some(&format!("out-{}", artifact.name)))?;

            run_generator_script(
                &artifact,
                &make.make_base.clone(),
                &prompt.path_buf,
                &out.path_buf,
            )
            .context("running generator script")?;

            verify_generated_files(artifact, &out.path_buf)?;

            run_serialize(&artifact, &backend, &out.path_buf, &machine)?
        }
    }
    Ok(())
}

pub(crate) fn run_check_serialization(
    artifact: &ArtifactDef,
    machine: &str,
    backend: &BackendConfiguration,
    make_base: &Path,
) -> anyhow::Result<bool> {
    let backend_name = &artifact.serialization;
    let backend_entry = backend.get_backend(backend_name)?;
    let artifact_name = sanitize_name(&artifact.name);

    let inputs = create_temp_dir(Some(&format!("inputs-{}", artifact_name)))?;

    for file in artifact.files.values() {
        let file_name = sanitize_name(&file.name);
        let resolved_path = resolve_path(make_base, &file.path);
        let json_path = inputs.path_buf.join(file_name);

        let text = to_string_pretty(&json!({
            "path": resolved_path,
            "owner": file.owner,
            "group": file.group,
        }))?;

        fs::write(&json_path, text).with_context(|| format!("writing {}", json_path.display()))?;
    }

    // Run check_serialization
    let check_path = resolve_path(&backend.base_path, &backend_entry.check_serialization);
    let check_abs = fs::canonicalize(&check_path).unwrap_or_else(|_| check_path.clone());

    debug!(
        "    running check_serialization: env inputs=\"{}\" machine=\"{}\" artifact=\"{}\" {}",
        inputs.path_buf.display(),
        machine,
        artifact.name,
        check_abs.display()
    );

    let status = std::process::Command::new(&check_abs)
        .env("inputs", &inputs.path_buf)
        .env("machine", machine)
        .env("artifact", &artifact.name)
        .status()
        .with_context(|| format!("running check_serialization {}", check_abs.display()))?;

    if status.success() {
        debug!(
            "    check_serialization: OK (exit 0) -> skipping generation and serialization for this artifact"
        );
        Ok(true)
    } else {
        debug!(
            "    check_serialization: failed with status exit status: {} -> continuing with generation",
            status.code().unwrap_or(0)
        );
        Ok(false)
    }
}

/// Converts a string into a safe filename by replacing non-alphanumeric characters with underscores.
///
/// # Arguments
///
/// * `name` - The string to sanitize
///
/// # Returns
///
/// A new String containing only ASCII alphanumeric characters and underscores.
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

pub fn run_regenerate_command(
    backend_toml: &Path,
    make_json: &Path,
    all: bool,
    machines_to_regenerate: &Vec<String>,
    artifacts_to_regenerate: &Vec<String>,
) -> Result<()> {
    // Validate argument rules
    if all {
        if !machines_to_regenerate.is_empty() || !artifacts_to_regenerate.is_empty() {
            bail!("--all conflicts with --machine/--artifact");
        }
    } else if machines_to_regenerate.is_empty() && artifacts_to_regenerate.is_empty() {
        bail!("provide --all or at least one of --machine/--artifact");
    }

    run_generate_workflow(
        backend_toml,
        make_json,
        all,
        machines_to_regenerate,
        artifacts_to_regenerate,
        false,
        "[regenerate]",
    )
}
