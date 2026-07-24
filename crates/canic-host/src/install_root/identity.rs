use super::{options::InstallRootOptions, truth_check::validate_expected_app_id};
use canic_core::ids::FleetName;
use std::path::Path;

pub(super) fn resolve_install_identity(
    options: &InstallRootOptions,
    config_path: &Path,
    app_id: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    validate_expected_app_id(options.expected_app.as_deref(), app_id, config_path)?;
    let fleet_name = options.fleet_name.parse::<FleetName>()?;
    Ok((app_id.to_string(), fleet_name.to_string()))
}
