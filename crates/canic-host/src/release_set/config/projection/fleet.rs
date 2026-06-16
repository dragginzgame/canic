use canic_core::bootstrap::parse_config_model;
use toml::Value as TomlValue;

// Read the required operator fleet name from raw config source.
pub(in crate::release_set) fn configured_fleet_name_from_source(
    config_source: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = toml::from_str::<TomlValue>(config_source)?;
    let name = config
        .get("fleet")
        .and_then(TomlValue::as_table)
        .and_then(|fleet| fleet.get("name"))
        .and_then(TomlValue::as_str)
        .ok_or_else(|| "missing required [fleet].name in canic.toml".to_string())?;
    Ok(name.to_string())
}

// Enumerate configured top-level deployment controllers from raw config source.
pub(in crate::release_set) fn configured_controllers_from_source(
    config_source: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut controllers = config
        .controllers
        .iter()
        .map(canic_core::cdk::types::Principal::to_text)
        .collect::<Vec<_>>();
    controllers.sort();
    controllers.dedup();
    Ok(controllers)
}
