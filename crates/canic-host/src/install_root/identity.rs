use super::{
    options::InstallRootOptions, state::validate_state_name,
    truth_check::validate_expected_fleet_name,
};
use std::path::Path;

pub(super) fn resolve_install_identity(
    options: &InstallRootOptions,
    config_path: &Path,
    fleet_name: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    validate_expected_fleet_name(options.expected_fleet.as_deref(), fleet_name, config_path)?;
    validate_state_name(fleet_name)?;
    let deployment_name = options
        .deployment_name
        .clone()
        .unwrap_or_else(|| fleet_name.to_string());
    validate_state_name(&deployment_name)?;
    Ok((fleet_name.to_string(), deployment_name))
}
