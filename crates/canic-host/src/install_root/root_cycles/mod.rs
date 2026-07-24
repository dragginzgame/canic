use crate::release_set::AppConfigSnapshot;
use std::{path::Path, process::Command};

pub(super) fn add_local_root_create_cycles_arg(
    command: &mut Command,
    config_path: &Path,
    environment: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if environment != "local" {
        return Ok(());
    }

    let cycles = AppConfigSnapshot::load(config_path)?.local_root_create_cycles();
    command.args(["--cycles", &cycles.to_string()]);
    Ok(())
}
