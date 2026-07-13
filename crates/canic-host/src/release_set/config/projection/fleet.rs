use super::parse_projection_config;
use crate::release_set::config::{
    FleetConfigDeclaration, FleetConfigError, FleetConfigTomlOperation,
};
use toml::Value as TomlValue;

// Read the required operator fleet name from raw config source.
pub(in crate::release_set) fn configured_fleet_name_from_source(
    config_source: &str,
) -> Result<String, FleetConfigError> {
    let config =
        toml::from_str::<TomlValue>(config_source).map_err(|source| FleetConfigError::Toml {
            operation: FleetConfigTomlOperation::ParseFleetIdentity,
            source,
        })?;
    let name = config
        .get("fleet")
        .and_then(TomlValue::as_table)
        .and_then(|fleet| fleet.get("name"))
        .and_then(TomlValue::as_str)
        .ok_or(FleetConfigError::DeclarationMissing {
            declaration: FleetConfigDeclaration::FleetName,
        })?;
    Ok(name.to_string())
}

// Enumerate configured top-level deployment controllers from raw config source.
pub(in crate::release_set) fn configured_controllers_from_source(
    config_source: &str,
) -> Result<Vec<String>, FleetConfigError> {
    let config = parse_projection_config(config_source)?;
    let mut controllers = config
        .controllers
        .iter()
        .map(canic_core::cdk::types::Principal::to_text)
        .collect::<Vec<_>>();
    controllers.sort();
    controllers.dedup();
    Ok(controllers)
}
