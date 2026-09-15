//! Module: fleet_ensure::generate::startup_funding
//!
//! Responsibility: bind the pure startup scenario to generated placements and native observations.
//! Does not own: authoritative grant history, funding approval or changes to desired policy.
//! Boundary: generation exposes assumptions; only a reviewed ensure plan may authorize funding.

use super::{DesiredFleet, FleetGenerateError, ObservedCanister};
use crate::fleet_ensure::{
    policy::startup_funding::{add, minimum_root_cycles, root_components},
    view::startup_funding::{
        StartupCoordinatorUsage, StartupFundingForecast, StartupNativeBalance, StartupRootFunding,
        StartupUsageUnavailable,
    },
};
use canic_core::bootstrap::compiled::ConfigModel;
use std::collections::BTreeMap;

/// Observe Root-funded Workloads and explicitly separate assets requiring a parent relay.
pub(super) fn observe_children(
    request: &super::FleetGenerateRequest<'_>,
    desired: &DesiredFleet,
    observed: &BTreeMap<String, ObservedCanister>,
    selected_module: &str,
    local_replica: Option<&crate::icp::LocalReplicaTarget>,
    forecast: &mut StartupFundingForecast,
) {
    use crate::fleet_ensure::view::startup_funding::StartupChildFundingUsage;
    let Some(protocol) = &desired.protocol else {
        return;
    };
    let icp = crate::icp::IcpCli::new(request.icp_executable, Some(request.environment.to_owned()))
        .with_cwd(request.root.to_path_buf())
        .with_local_replica(local_replica.cloned());
    let identity_matches = icp.bind_selected_identity().is_ok()
        && icp
            .identity_principal_text()
            .is_ok_and(|principal| principal == desired.operator);
    for root in &mut forecast.roots {
        let parent = desired
            .canisters
            .iter()
            .find(|canister| canister.name == root.root)
            .and_then(|canister| canister.principal.as_deref())
            .and_then(|principal| principal.parse::<candid::Principal>().ok());
        let Some(parent) = parent else {
            continue;
        };
        let installed = observed
            .get(&parent.to_text())
            .and_then(|canister| canister.module_sha256.as_deref())
            == Some(selected_module);
        for canister in &desired.canisters {
            if canister.kind != crate::fleet_ensure::model::DesiredCanisterKind::Pool
                || canister.parent.as_deref() != Some(root.root.as_str())
            {
                continue;
            }
            let Some(child) = canister.principal.as_deref() else {
                continue;
            };
            let binding =
                if !identity_matches {
                    Err(StartupUsageUnavailable::AuthorityMismatch)
                } else if !installed {
                    Err(StartupUsageUnavailable::SelectedBuildNotInstalled)
                } else if let Ok(child) = child.parse::<candid::Principal>() {
                    observed.get(&child.to_text())
                    .and_then(|value| value.pool_status.as_ref())
                    .ok_or(StartupUsageUnavailable::NotObserved)
                    .and_then(|status| {
                        crate::fleet_ensure::ops::startup_funding::observation::binding::observe(
                            &icp, &request.root.join(&protocol.root_candid), parent, child, status,
                        )
                    })
                } else {
                    Err(StartupUsageUnavailable::AuthorityMismatch)
                };
            let usage = match &binding {
                Ok(binding) if binding.parent == parent.to_text() => {
                    crate::fleet_ensure::ops::startup_funding::observation::observe_child(
                        &icp,
                        &request.root.join(&protocol.root_candid),
                        parent,
                        child
                            .parse()
                            .expect("binding observation validated child Principal"),
                    )
                }
                Ok(_) => Err(StartupUsageUnavailable::ParentRelayRequired),
                Err(reason) => Err(*reason),
            };
            root.child_usage.push(StartupChildFundingUsage {
                allowance: Err(StartupUsageUnavailable::NotObserved),
                binding: binding.ok(),
                name: canister.name.clone(),
                child: child.into(),
                usage,
            });
        }
    }
}

/// Interpret live charges only through the exact selected release and compiled Spec policy.
pub(super) fn apply_allowances(
    config: &ConfigModel,
    desired: &DesiredFleet,
    forecast: &mut StartupFundingForecast,
) {
    let Some(bootstrap) = &desired.bootstrap else {
        return;
    };
    for root in &mut forecast.roots {
        for child in &mut root.child_usage {
            child.allowance = match (&child.binding, &child.usage) {
                (Some(binding), Ok(usage)) => {
                    crate::fleet_ensure::policy::startup_funding::allowance::project(
                        config,
                        &bootstrap
                            .component_deployment_configuration
                            .component_topology,
                        bootstrap.release_build_id,
                        binding,
                        usage,
                    )
                }
                (_, Err(reason)) => Err(*reason),
                (None, Ok(_)) => Err(StartupUsageUnavailable::AuthorityMismatch),
            };
        }
    }
}

/// Observe live Coordinator accounting only when the selected module is installed.
pub(super) fn observe_usage(
    request: &super::FleetGenerateRequest<'_>,
    desired: &DesiredFleet,
    observed: &BTreeMap<String, ObservedCanister>,
    selected_module: &str,
    local_replica: Option<&crate::icp::LocalReplicaTarget>,
) -> StartupCoordinatorUsage {
    let unavailable = StartupCoordinatorUsage::Unavailable;
    let (Some(bootstrap), Some(protocol)) = (&desired.bootstrap, &desired.protocol) else {
        return unavailable(StartupUsageUnavailable::NotObserved);
    };
    let Some(policy) = bootstrap.root_funding.as_ref() else {
        return unavailable(StartupUsageUnavailable::NotObserved);
    };
    let principal = |name: &str| {
        desired
            .canisters
            .iter()
            .find(|canister| canister.name == name)
            .and_then(|canister| canister.principal.as_deref())
            .and_then(|principal| principal.parse::<candid::Principal>().ok())
    };
    let Some(coordinator) = principal(&bootstrap.coordinator) else {
        return unavailable(StartupUsageUnavailable::NotObserved);
    };
    if observed
        .get(&coordinator.to_text())
        .and_then(|canister| canister.module_sha256.as_deref())
        != Some(selected_module)
    {
        return unavailable(StartupUsageUnavailable::SelectedBuildNotInstalled);
    }
    let Some(roots) = bootstrap
        .roots
        .iter()
        .map(|root| principal(&root.root))
        .collect::<Option<std::collections::BTreeSet<_>>>()
    else {
        return unavailable(StartupUsageUnavailable::AuthorityMismatch);
    };
    if roots.len() != bootstrap.roots.len() {
        return unavailable(StartupUsageUnavailable::AuthorityMismatch);
    }
    let icp = crate::icp::IcpCli::new(request.icp_executable, Some(request.environment.to_owned()))
        .with_cwd(request.root.to_path_buf())
        .with_local_replica(local_replica.cloned());
    if icp.bind_selected_identity().is_err()
        || !icp
            .identity_principal_text()
            .is_ok_and(|principal| principal == desired.operator)
    {
        return unavailable(StartupUsageUnavailable::AuthorityMismatch);
    }
    crate::fleet_ensure::ops::startup_funding::observation::observe(
        &icp,
        &request.root.join(&protocol.coordinator_candid),
        coordinator,
        &roots,
        policy,
    )
}

pub(super) fn forecast(
    config: &ConfigModel,
    desired: &DesiredFleet,
    observed: &BTreeMap<String, ObservedCanister>,
) -> Result<StartupFundingForecast, FleetGenerateError> {
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or_else(|| missing("bootstrap"))?;
    let coordinator_policy = bootstrap
        .root_funding
        .as_ref()
        .ok_or_else(|| missing("Coordinator funding"))?;
    let coordinator_balance = native_balance(desired, observed, &bootstrap.coordinator)?;
    let coordinator_reserve_cycles = coordinator_policy.minimum_reserve_cycles.to_u128();
    let mut roots = Vec::new();
    for root in &bootstrap.roots {
        let components = root_components(config, desired, root)?;
        let child_grants_cycles = components.iter().try_fold(0, |total, component| {
            add(total, component.root_grant_cycles)
        })?;
        let request_threshold_cycles = root.funding.root_funding.request_threshold.to_u128();
        let minimum_native_cycles =
            minimum_root_cycles(request_threshold_cycles, child_grants_cycles)?;
        let balance = native_balance(desired, observed, &root.root)?;
        roots.push(StartupRootFunding {
            child_usage: Vec::new(),
            balance,
            child_grants_cycles,
            components,
            exceeds_window_budget: child_grants_cycles
                > root.limits.cycles_funding.maximum_cycles.to_u128(),
            funding_budget: root.limits.cycles_funding.clone(),
            minimum_native_cycles,
            request_threshold_cycles,
            root: root.root.clone(),
            shortfall_cycles: minimum_native_cycles.saturating_sub(balance.cycles()),
        });
    }
    Ok(StartupFundingForecast {
        coordinator_usage: StartupCoordinatorUsage::Unavailable(
            StartupUsageUnavailable::NotObserved,
        ),
        coordinator_balance,
        coordinator_reserve_cycles,
        coordinator_spendable_cycles: coordinator_balance
            .cycles()
            .saturating_sub(coordinator_reserve_cycles),
        roots,
    })
}

fn native_balance(
    desired: &DesiredFleet,
    observed: &BTreeMap<String, ObservedCanister>,
    name: &str,
) -> Result<StartupNativeBalance, FleetGenerateError> {
    let canister = desired
        .canisters
        .iter()
        .find(|canister| canister.name == name)
        .ok_or_else(|| missing("infrastructure canister"))?;
    if let Some(principal) = &canister.principal {
        return observed
            .get(principal)
            .map(|observed| StartupNativeBalance::Observed(observed.cycles))
            .ok_or_else(|| missing("native balance observation"));
    }
    canister
        .initial_cycles
        .parse::<canic_core::cdk::types::Cycles>()
        .map(|cycles| StartupNativeBalance::ConfiguredCreation(cycles.to_u128()))
        .map_err(|error| {
            FleetGenerateError::Authority(format!("invalid startup creation balance: {error}"))
        })
}

fn missing(subject: &str) -> FleetGenerateError {
    FleetGenerateError::Authority(format!("startup funding forecast requires {subject}"))
}
