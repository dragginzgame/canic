use super::super::*;
use super::shared::observation_gap;
use crate::release_set::{
    ConfiguredPoolExpectation, configured_deployable_roles, configured_fleet_name,
    configured_pool_expectations,
};
use std::path::Path;

const UNKNOWN_FLEET_NAME: &str = "unknown";

pub(super) struct LocalConfigObservation {
    pub(super) fleet_name: String,
    pub(super) roles: Vec<String>,
    pub(super) pool_expectations: Vec<ConfiguredPoolExpectation>,
}

pub(super) fn observe_local_config_facts(
    config: &Path,
    unresolved_observations: &mut Vec<DeploymentObservationGapV1>,
) -> LocalConfigObservation {
    let fleet_name = configured_fleet_name(config).unwrap_or_else(|err| {
        unresolved_observations.push(observation_gap(
            "local_config.fleet_name",
            format!(
                "could not resolve fleet name from {}: {err}",
                config.display()
            ),
        ));
        UNKNOWN_FLEET_NAME.to_string()
    });
    let roles = configured_deployable_roles(config).map_or_else(
        |err| {
            unresolved_observations.push(observation_gap(
                "local_config.roles",
                format!(
                    "could not resolve configured roles from {}: {err}",
                    config.display()
                ),
            ));
            Vec::new()
        },
        deployment_truth_roles_with_implicit_wasm_store,
    );
    let pool_expectations = configured_pool_expectations(config).unwrap_or_else(|err| {
        unresolved_observations.push(observation_gap(
            "local_config.pools",
            format!(
                "could not resolve configured pool expectations from {}: {err}",
                config.display()
            ),
        ));
        Vec::new()
    });
    LocalConfigObservation {
        fleet_name,
        roles,
        pool_expectations,
    }
}
