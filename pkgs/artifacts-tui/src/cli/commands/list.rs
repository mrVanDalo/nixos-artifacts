use crate::config::make::MakeConfiguration;
use anyhow::Result;
use log::info;
use std::path::Path;

/// List all machines and artifacts configured in make.json
pub fn run(make_json: &Path) -> Result<()> {
    let make = MakeConfiguration::read_make_config(make_json)?;

    for (machine, artifacts) in &make.make_map {
        info!("[list]");
        info!("machine: {}", machine);
        for artifact in artifacts {
            info!("artifact: {}", artifact.name);
        }
    }

    Ok(())
}
