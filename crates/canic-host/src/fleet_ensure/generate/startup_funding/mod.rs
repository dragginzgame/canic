//! Module: fleet_ensure::generate::startup_funding
//!
//! Responsibility: bind the pure startup scenario to generated placements and native observations.
//! Does not own: authoritative grant history, funding approval or changes to desired policy.
//! Boundary: generation exposes assumptions; only a reviewed ensure plan may authorize funding.

use super::{DesiredFleet, FleetGenerateError, ObservedCanister};
use crate::{
    fleet_ensure::{
        ops::startup_funding::observation::{self, binding, inventory, registry},
        policy::startup_funding::{add, live_binding, minimum_root_cycles, root_components},
        view::startup_funding::{
            StartupChildFundingUsage, StartupCoordinatorUsage, StartupDemandUnavailable,
            StartupFundingForecast, StartupFundingRegistry, StartupInventoryCoverage,
            StartupNativeBalance, StartupRootFunding, StartupRootInventory,
            StartupUsageUnavailable,
        },
    },
    icp::{IcpCli, LocalReplicaTarget},
};
use candid::Principal;
use canic_core::{bootstrap::compiled::ConfigModel, dto::pool::CanisterPoolAssetStatus};
use std::{collections::BTreeMap, path::Path};

/// Observe Root-funded Workloads and explicitly separate assets requiring a parent relay.
pub(super) fn observe_children(
    request: &super::FleetGenerateRequest<'_>,
    desired: &DesiredFleet,
    observed: &BTreeMap<String, ObservedCanister>,
    selected_module: &str,
    selected_coordinator_module: &str,
    local_replica: Option<&LocalReplicaTarget>,
    forecast: &mut StartupFundingForecast,
) {
    let Some(protocol) = &desired.protocol else {
        return;
    };
    let icp = IcpCli::new(request.icp_executable, Some(request.environment.to_owned()))
        .with_identity(request.signing_identity)
        .with_cwd(request.root.to_path_buf())
        .with_local_replica(local_replica.cloned());
    let identity_matches = icp.bind_selected_identity().is_ok()
        && icp
            .identity_principal_text()
            .is_ok_and(|principal| principal == desired.operator);
    let registry = if identity_matches {
        observe_registry(
            request,
            desired,
            observed,
            selected_coordinator_module,
            &icp,
        )
    } else {
        Err(StartupUsageUnavailable::AuthorityMismatch)
    };
    for root in &mut forecast.roots {
        let parent = desired
            .canisters
            .iter()
            .find(|canister| canister.name == root.root)
            .and_then(|canister| canister.principal.as_deref())
            .and_then(|principal| principal.parse::<Principal>().ok());
        let Some(parent) = parent else {
            continue;
        };
        let installed = observed
            .get(&parent.to_text())
            .and_then(|canister| canister.module_sha256.as_deref())
            == Some(selected_module);
        let root_candid = request.root.join(&protocol.root_candid);
        let root_registry = if installed {
            registry
                .as_ref()
                .map_err(|reason| *reason)
                .and_then(|registry| {
                    let summary = registry::observe_root(&icp, &root_candid, parent, registry)?;
                    Ok((registry, summary))
                })
        } else {
            Err(StartupUsageUnavailable::SelectedBuildNotInstalled)
        };
        observe_root_children(
            &icp,
            &root_candid,
            desired,
            observed,
            root,
            parent,
            root_registry.map(|(registry, _)| registry),
        );
        root.inventory = root_registry.and_then(|(registry, summary)| {
            qualify_and_observe_usage(&icp, &root_candid, desired, root, parent, registry, summary)
        });
        if let Err(reason) = root.inventory {
            invalidate_children(&mut root.child_usage, reason);
        }
    }
    if let Ok(registry) = registry
        && let Err(reason) = registry::recheck(
            &icp,
            &request.root.join(&protocol.coordinator_candid),
            &registry,
        )
    {
        for root in &mut forecast.roots {
            root.inventory = Err(reason);
            invalidate_children(&mut root.child_usage, reason);
        }
    }
}

fn invalidate_children(children: &mut [StartupChildFundingUsage], reason: StartupUsageUnavailable) {
    for child in children {
        if child.binding.take().is_some() {
            child.usage = Err(reason);
        }
    }
}

fn observe_root_children(
    icp: &IcpCli,
    candid: &Path,
    desired: &DesiredFleet,
    observed: &BTreeMap<String, ObservedCanister>,
    root: &mut StartupRootFunding,
    parent: Principal,
    registry: Result<&StartupFundingRegistry, StartupUsageUnavailable>,
) {
    for canister in &desired.canisters {
        if canister.kind != crate::fleet_ensure::model::DesiredCanisterKind::Pool
            || canister.parent.as_deref() != Some(root.root.as_str())
        {
            continue;
        }
        let Some(child) = canister.principal.as_deref() else {
            continue;
        };
        let binding = observed
            .get(child)
            .and_then(|value| value.pool_status.as_ref())
            .ok_or(StartupUsageUnavailable::NotObserved)
            .and_then(|status| {
                if !matches!(status, CanisterPoolAssetStatus::Workload { .. }) {
                    return Err(StartupUsageUnavailable::NotWorkload);
                }
                let registry = registry?;
                let binding = binding::observe(
                    icp,
                    candid,
                    parent,
                    child
                        .parse()
                        .map_err(|_| StartupUsageUnavailable::AuthorityMismatch)?,
                    status,
                )?;
                live_binding::current(desired, &root.root, &binding, registry)?;
                Ok(binding)
            });
        let usage = Err(binding
            .as_ref()
            .err()
            .copied()
            .unwrap_or(StartupUsageUnavailable::NotObserved));
        root.child_usage.push(StartupChildFundingUsage {
            allowance: Err(StartupUsageUnavailable::NotObserved),
            observed_balance_cycles: observed.get(child).map(|value| value.cycles),
            local_demand: Err(StartupDemandUnavailable::Usage(
                StartupUsageUnavailable::NotObserved,
            )),
            binding: binding.ok(),
            name: canister.name.clone(),
            child: child.into(),
            usage,
        });
    }
}

fn qualify_and_observe_usage(
    icp: &IcpCli,
    candid: &Path,
    desired: &DesiredFleet,
    root: &mut StartupRootFunding,
    parent: Principal,
    registry: &StartupFundingRegistry,
    summary: StartupRootInventory,
) -> Result<StartupInventoryCoverage, StartupUsageUnavailable> {
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or(StartupUsageUnavailable::NotObserved)?;
    let topology = &bootstrap
        .component_deployment_configuration
        .component_topology;
    let bindings = root
        .child_usage
        .iter()
        .filter(|child| child.usage != Err(StartupUsageUnavailable::NotWorkload))
        .map(|child| {
            child.binding.as_ref().ok_or_else(|| {
                child
                    .usage
                    .as_ref()
                    .err()
                    .copied()
                    .unwrap_or(StartupUsageUnavailable::InventoryIncomplete)
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    for result in live_binding::graph(
        topology,
        bootstrap.release_build_id,
        bindings.iter().copied(),
    )
    .values()
    {
        (*result)?;
    }
    let placement = registry
        .roots
        .get(&parent)
        .ok_or(StartupUsageUnavailable::AuthorityMismatch)?;
    let evidence =
        inventory::observe(icp, candid, parent, placement, topology, summary, &bindings)?;
    for child in &mut root.child_usage {
        if let Some(binding) = &child.binding {
            child.usage = if binding.parent == parent {
                observation::observe_child(icp, candid, parent, binding.canister_id)
            } else {
                Err(StartupUsageUnavailable::ParentRelayRequired)
            };
        }
    }
    let coverage = inventory::recheck(icp, candid, parent, &evidence)?;
    if registry::observe_root(icp, candid, parent, registry)? != summary {
        return Err(StartupUsageUnavailable::PolicyTransition);
    }
    Ok(coverage)
}

fn observe_registry(
    request: &super::FleetGenerateRequest<'_>,
    desired: &DesiredFleet,
    observed: &BTreeMap<String, ObservedCanister>,
    selected_module: &str,
    icp: &IcpCli,
) -> Result<StartupFundingRegistry, StartupUsageUnavailable> {
    let bootstrap = desired
        .bootstrap
        .as_ref()
        .ok_or(StartupUsageUnavailable::NotObserved)?;
    let protocol = desired
        .protocol
        .as_ref()
        .ok_or(StartupUsageUnavailable::NotObserved)?;
    let coordinator = desired
        .canisters
        .iter()
        .find(|canister| canister.name == bootstrap.coordinator)
        .and_then(|canister| canister.principal.as_deref())
        .ok_or(StartupUsageUnavailable::NotObserved)?;
    if observed
        .get(coordinator)
        .and_then(|canister| canister.module_sha256.as_deref())
        != Some(selected_module)
    {
        return Err(StartupUsageUnavailable::SelectedBuildNotInstalled);
    }
    registry::observe(
        icp,
        &request.root.join(&protocol.coordinator_candid),
        coordinator
            .parse()
            .map_err(|_| StartupUsageUnavailable::AuthorityMismatch)?,
        &bootstrap
            .component_deployment_configuration
            .component_topology,
    )
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
        root.recovery_demand =
            crate::fleet_ensure::policy::startup_funding::recursive::project(config, desired, root);
        root.relay_quote =
            crate::fleet_ensure::policy::startup_funding::relay_quote::project(desired, root);
        let bindings = live_binding::graph(
            &bootstrap
                .component_deployment_configuration
                .component_topology,
            bootstrap.release_build_id,
            root.child_usage
                .iter()
                .filter_map(|child| child.binding.as_ref()),
        );
        for child in &mut root.child_usage {
            if let Some(binding) = &child.binding
                && let Some(Err(reason)) = bindings.get(&binding.canister_id)
            {
                child.allowance = Err(*reason);
                child.local_demand = Err(StartupDemandUnavailable::Usage(*reason));
                continue;
            }
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
            child.local_demand = match (&child.binding, &child.usage) {
                (Some(binding), Ok(usage)) => {
                    crate::fleet_ensure::policy::startup_funding::local_demand::project(
                        config,
                        &bootstrap
                            .component_deployment_configuration
                            .component_topology,
                        bootstrap.release_build_id,
                        binding,
                        usage,
                        child.observed_balance_cycles,
                    )
                }
                (_, Err(reason)) => Err(StartupDemandUnavailable::Usage(*reason)),
                (None, Ok(_)) => Err(StartupDemandUnavailable::Usage(
                    StartupUsageUnavailable::AuthorityMismatch,
                )),
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
    local_replica: Option<&LocalReplicaTarget>,
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
            .and_then(|principal| principal.parse::<Principal>().ok())
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
    let icp = IcpCli::new(request.icp_executable, Some(request.environment.to_owned()))
        .with_identity(request.signing_identity)
        .with_cwd(request.root.to_path_buf())
        .with_local_replica(local_replica.cloned());
    if icp.bind_selected_identity().is_err()
        || !icp
            .identity_principal_text()
            .is_ok_and(|principal| principal == desired.operator)
    {
        return unavailable(StartupUsageUnavailable::AuthorityMismatch);
    }
    observation::observe(
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
            recovery_demand: Err(StartupDemandUnavailable::BalanceNotObserved),
            relay_quote: Err(StartupDemandUnavailable::Usage(
                StartupUsageUnavailable::NotObserved,
            )),
            inventory: Err(StartupUsageUnavailable::NotObserved),
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
