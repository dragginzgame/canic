//! Module: fleet_ensure::ops::startup_funding
//!
//! Responsibility: bind startup demand to the reviewed application configuration.
//! Does not own: native observations, spending approval or funding effects.
//! Boundary: recomputed topology must equal desired authority before policy consumes demand.

pub mod observation;

use crate::{
    fleet_ensure::{
        model::{DesiredFleet, StartupFundingRequirement, StartupRoleShortfall},
        ops::EnsureStateError,
        policy::startup_funding::{add, minimum_root_cycles, root_components},
    },
    release_set::AppConfigSnapshot,
};
use std::{collections::BTreeMap, path::Path};

pub(super) fn resolve(
    workspace: &Path,
    desired: &DesiredFleet,
) -> Result<BTreeMap<String, StartupFundingRequirement>, EnsureStateError> {
    let (Some(bootstrap), Some(protocol)) = (&desired.bootstrap, &desired.protocol) else {
        return Ok(BTreeMap::new());
    };
    let config = AppConfigSnapshot::load(&workspace.join(&protocol.app_config))
        .map_err(|error| EnsureStateError::StartupConfiguration(Box::new(error)))?;
    let configuration = config
        .model()
        .compile_component_deployment_configuration()
        .map_err(|_| EnsureStateError::StartupConfigurationMismatch)?;
    if configuration != bootstrap.component_deployment_configuration {
        return Err(EnsureStateError::StartupConfigurationMismatch);
    }
    let mut requirements = BTreeMap::new();
    for root in &bootstrap.roots {
        let components = root_components(config.model(), desired, root).map_err(invalid)?;
        let grants = components
            .iter()
            .try_fold(0, |total, component| {
                add(total, component.root_grant_cycles)
            })
            .map_err(invalid)?;
        let minimum_native_cycles = minimum_root_cycles(
            root.funding.root_funding.request_threshold.to_u128(),
            grants,
        )
        .map_err(invalid)?;
        let unfunded_role = components
            .iter()
            .flat_map(|component| &component.roles)
            .find(|role| role.unfunded_per_instance_cycles > 0)
            .map(|role| StartupRoleShortfall {
                role: role.role.clone(),
                cycles: role.unfunded_per_instance_cycles,
            });
        requirements.insert(
            root.root.clone(),
            StartupFundingRequirement {
                maximum_continuation_steps: 0,
                minimum_native_cycles,
                unfunded_role,
            },
        );
    }
    Ok(requirements)
}

fn invalid(error: crate::fleet_ensure::policy::EnsurePolicyError) -> EnsureStateError {
    EnsureStateError::StartupFunding(Box::new(error))
}
