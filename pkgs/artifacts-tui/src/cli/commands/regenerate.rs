use crate::backend::generator::GeneratorManger;
use crate::backend::helpers::print_files;
use crate::backend::helpers::resolve_path;
use crate::backend::prompt::PromptManager;
use crate::backend::serialization::run_serialize;
use crate::backend::temp_dir::create_temp_dir;
use crate::config::backend::BackendConfiguration;
use crate::config::make::{ArtifactDef, MakeConfiguration};
use anyhow::{Context, Result, bail};
use log::info;
use std::fs;
use std::path::Path;

pub fn run(
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

    let backend = BackendConfiguration::read_backend_config(backend_toml)?;
    let make = MakeConfiguration::read_make_config(make_json)?;

    let prompt_manager = PromptManager::new();
    let generator_manager = GeneratorManger::new();

    let check_all = || -> bool { all };

    let check_machine = |machine: &str| -> bool {
        machines_to_regenerate.is_empty() || machines_to_regenerate.iter().any(|m| m == machine)
    };

    let check_artifact = |artifact: &ArtifactDef| -> bool {
        artifacts_to_regenerate.is_empty()
            || artifacts_to_regenerate.iter().any(|a| a == &artifact.name)
    };

    for (machine, artifacts) in &make.make_map {
        for artifact in artifacts {
            if !(check_all() || check_machine(machine) && check_artifact(&artifact)) {
                continue;
            }

            info!("[regenerate]");
            info!("machine: {}", machine);
            info!("artifact: {}", artifact.name);
            print_files(&artifact, &make.make_base);

            let prompt_results = prompt_manager
                .query_prompts(&artifact)
                .context("could not query all prompts")?;

            let prompt = create_temp_dir(Some(&format!("prompt-{}", artifact.name)))?;
            prompt_results
                .write_prompts_to_files(&prompt.path_buf)
                .context("writing prompts to files")?;

            let out = create_temp_dir(Some(&format!("out-{}", artifact.name)))?;

            generator_manager
                .run_generator_script(
                    &artifact,
                    &make.make_base.clone(),
                    &prompt.path_buf,
                    &out.path_buf,
                )
                .context("running generator script")?;

            generator_manager.verify_generated_files(artifact, &out.path_buf)?;

            run_serialize(&artifact, &backend, &out.path_buf, &machine)?
        }
    }

    Ok(())
}
