use crate::release_set::config::{
    FleetConfigDeclaration, FleetConfigError, FleetConfigTomlOperation,
};
use canic_core::bootstrap::compiled::ConfigModel;
use toml::Value as TomlValue;

// Read only the identity required to deduplicate incomplete discovery candidates.
pub(in crate::release_set) fn fleet_identity_from_source(
    config_source: &str,
) -> Result<String, FleetConfigError> {
    let config =
        toml::from_str::<TomlValue>(config_source).map_err(|source| FleetConfigError::Toml {
            operation: FleetConfigTomlOperation::ParseFleetIdentity,
            source,
        })?;
    config
        .get("fleet")
        .and_then(TomlValue::as_table)
        .and_then(|fleet| fleet.get("name"))
        .and_then(TomlValue::as_str)
        .map(str::to_string)
        .ok_or(FleetConfigError::DeclarationMissing {
            declaration: FleetConfigDeclaration::FleetName,
        })
}

// Enumerate configured top-level deployment controllers from one validated snapshot.
pub(in crate::release_set) fn configured_controllers_from_config(
    config: &ConfigModel,
) -> Vec<String> {
    let mut controllers = config
        .controllers
        .iter()
        .map(canic_core::cdk::types::Principal::to_text)
        .collect::<Vec<_>>();
    controllers.sort();
    controllers.dedup();
    controllers
}
