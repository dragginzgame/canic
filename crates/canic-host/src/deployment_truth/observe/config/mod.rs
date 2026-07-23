use super::super::*;
use super::shared::observation_gap;
use crate::release_set::{AppConfigSnapshot, ConfiguredPoolExpectation};
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
    let (fleet_name, roles, pool_expectations) = match AppConfigSnapshot::load(config) {
        Ok(snapshot) => (
            snapshot.app_id().to_string(),
            deployment_truth_roles_with_implicit_wasm_store(snapshot.deployable_roles()),
            snapshot.pool_expectations(),
        ),
        Err(err) => {
            for (code, subject) in [
                ("local_config.fleet_name", "fleet name"),
                ("local_config.roles", "configured roles"),
                ("local_config.pools", "configured pool expectations"),
            ] {
                unresolved_observations.push(observation_gap(
                    code,
                    format!(
                        "could not resolve {subject} from {}: {err}",
                        config.display()
                    ),
                ));
            }
            (UNKNOWN_FLEET_NAME.to_string(), Vec::new(), Vec::new())
        }
    };
    LocalConfigObservation {
        fleet_name,
        roles,
        pool_expectations,
    }
}
