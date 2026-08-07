use super::super::*;
use super::shared::observation_gap;
use crate::release_set::AppConfigSnapshot;
use std::path::Path;

const UNKNOWN_APP: &str = "unknown";

pub(super) struct LocalConfigObservation {
    pub(super) app: String,
    pub(super) roles: Vec<String>,
}

pub(super) fn observe_local_config_facts(
    config: &Path,
    unresolved_observations: &mut Vec<DeploymentObservationGapV1>,
) -> LocalConfigObservation {
    let (app, roles) = match AppConfigSnapshot::load(config) {
        Ok(snapshot) => (
            snapshot.app_id().to_string(),
            deployment_truth_roles_with_built_in_infrastructure(snapshot.deployable_roles()),
        ),
        Err(err) => {
            for (code, subject) in [
                ("local_config.app", "App identity"),
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
            (UNKNOWN_APP.to_string(), Vec::new())
        }
    };
    LocalConfigObservation { app, roles }
}
