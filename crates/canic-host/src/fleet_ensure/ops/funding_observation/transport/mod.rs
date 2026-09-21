//! Module: fleet_ensure::ops::funding_observation::transport
//!
//! Responsibility: bind current ICP authority and perform a reviewed funding observation attempt.
//! Boundary: discovery uses read-only calls; paid Root commands require workflow-owned intent.

#[cfg(test)]
mod tests;

use crate::{
    canister_protocol::call_with_candid,
    fleet_ensure::{
        model::{
            CanisterRuntimeStatus, FleetEnsurePlan, MAX_FLEET_ENSURE_CANISTERS,
            ReviewedDesiredFleetRecord, funding_observation::*,
        },
        ops::{
            IcpEnsurePlatform, IcpEnsurePlatformError,
            current_inventory::{ProtocolCatalog, ProtocolEntry},
            startup_funding::observation::{self, binding, inventory, registry},
        },
        policy::startup_funding::live_binding,
        view::startup_funding::*,
        workflow::funding_observation::{FundingObservationPlatform, FundingObservationSnapshot},
    },
    release_set::AppConfigSnapshot,
};
use candid::{CandidType, Deserialize, Principal};
use canic_core::{
    cdk::utils::hash::hex_bytes,
    dto::{
        canister::{CanisterInspectionRequest, CanisterStatusResponse, CanisterStatusType},
        observability::{
            CanisterObservabilityRequest, CanisterObservabilityResponse,
            FleetCanisterObservabilityRequest,
        },
        pool::CanisterPoolAssetStatus,
    },
    protocol,
};
use std::collections::BTreeSet;

#[derive(CandidType)]
enum Command {
    InspectCanister(CanisterInspectionRequest),
    ObserveCanister(FleetCanisterObservabilityRequest),
}

#[derive(CandidType, Deserialize)]
enum Response {
    InspectCanister(Box<CanisterStatusResponse>),
    InspectionReserveRequired(canic_core::dto::canister::CanisterInspectionReserveResponse),
    ObserveCanister(CanisterObservabilityResponse),
}

impl FundingObservationPlatform for IcpEnsurePlatform {
    type Error = IcpEnsurePlatformError;

    fn snapshot(
        &mut self,
        plan: &FleetEnsurePlan,
        desired: &crate::fleet_ensure::model::DesiredFleet,
        root_name: &str,
    ) -> Result<FundingObservationSnapshot, Self::Error> {
        self.require_operator()?;
        require_plan(self, plan)?;
        let (config, catalog) = catalog(self)?;
        let bootstrap = desired
            .bootstrap
            .as_ref()
            .ok_or(FundingObservationError::Unavailable)?;
        let coordinator = principal(desired, &bootstrap.coordinator)?;
        let root_id = principal(desired, root_name)?;
        qualify_owner(self, desired, &bootstrap.coordinator, &catalog.coordinator)?;
        let balance = qualify_owner(self, desired, root_name, &catalog.root)?;
        let registry = registry::observe(
            &self.icp,
            &catalog.coordinator.candid_path,
            coordinator,
            config.component_topology(),
        )
        .map_err(unavailable)?;
        let summary =
            registry::observe_root(&self.icp, &catalog.root.candid_path, root_id, &registry)
                .map_err(unavailable)?;
        let placement = registry
            .roots
            .get(&root_id)
            .ok_or(FundingObservationError::AuthorityMismatch)?;
        let children = children(self, desired, root_name, root_id, &catalog.root, &registry)?;
        let bindings = children
            .iter()
            .filter_map(|child| child.binding.as_ref())
            .collect::<Vec<_>>();
        let evidence = inventory::observe(
            &self.icp,
            &catalog.root.candid_path,
            root_id,
            placement,
            config.component_topology(),
            summary,
            &bindings,
        )
        .map_err(unavailable)?;
        let coverage = inventory::recheck(&self.icp, &catalog.root.candid_path, root_id, &evidence)
            .map_err(unavailable)?;
        if registry::observe_root(&self.icp, &catalog.root.candid_path, root_id, &registry)
            .map_err(unavailable)?
            != summary
        {
            return Err(FundingObservationError::AuthorityMismatch.into());
        }
        registry::recheck(&self.icp, &catalog.coordinator.candid_path, &registry)
            .map_err(unavailable)?;
        Ok(FundingObservationSnapshot {
            configuration_source: config.source().to_owned(),
            authority: FundingObservationAuthorityRecord {
                registry: registry.authority,
                revision: registry.revision,
                content_hash: registry.content_hash,
                components: inventory::heads(&evidence),
            },
            root: StartupRootFunding {
                root: root_name.into(),
                balance: StartupNativeBalance::Observed(balance),
                child_usage: children,
                inventory: Ok(coverage),
                recovery_demand: Err(StartupDemandUnavailable::BalanceNotObserved),
                relay_quote: Err(StartupDemandUnavailable::BalanceNotObserved),
                child_grants_cycles: 0,
                components: Vec::new(),
                funding_budget: placement.limits.cycles_funding.clone(),
                exceeds_window_budget: false,
                minimum_native_cycles: 0,
                request_threshold_cycles: 0,
                shortfall_cycles: 0,
            },
        })
    }

    fn child_balance(
        &mut self,
        plan: &FleetEnsurePlan,
        request: &FundingObservationRequestRecord,
    ) -> Result<u128, Self::Error> {
        self.require_operator()?;
        require_plan(self, plan)?;
        let (_, catalog) = catalog(self)?;
        let root = request.component.fleet_subnet_root;
        let path = &catalog.root.candid_path;
        // Native inspection precedes a ledger relay and qualifies exact installed code/controllers.
        // A failure consumes the whole attempt; neither call is retried here.
        let response: Response = call_with_candid(
            &self.icp,
            path,
            root,
            protocol::CANIC_ROOT_COMMAND,
            &Command::InspectCanister(CanisterInspectionRequest {
                canister_id: request.child,
            }),
        )
        .map_err(crate::fleet_ensure::ops::current_protocol::CurrentProtocolError::from)?;
        let status = inspection_response(response, root, request.child)?;
        let entry = catalog
            .child(&request.role)
            .ok_or(FundingObservationError::AuthorityMismatch)?;
        let hash = status.module_hash.as_ref().map(hex_bytes);
        if status.status != CanisterStatusType::Running
            || status.settings.controllers != vec![root]
            || !hash.as_ref().is_some_and(|hash| {
                hash == &entry.raw_module_hash || hash == &entry.installed_module_hash
            })
        {
            return Err(FundingObservationError::AuthorityMismatch.into());
        }
        let native_cycles = u128::try_from(status.cycles.0)
            .map_err(|_| FundingObservationError::ArithmeticOverflow)?;
        Ok(native_cycles)
    }

    fn child_usage(
        &mut self,
        plan: &FleetEnsurePlan,
        request: &FundingObservationRequestRecord,
        native_cycles: u128,
    ) -> Result<FundingChildAccountingRecord, Self::Error> {
        self.require_operator()?;
        require_plan(self, plan)?;
        let (_, catalog) = catalog(self)?;
        let root = request.component.fleet_subnet_root;
        let path = &catalog.root.candid_path;
        let usage = if request.parent == root {
            observation::observe_child(&self.icp, path, root, request.child).map_err(unavailable)?
        } else {
            let response: Response = call_with_candid(
                &self.icp,
                path,
                root,
                protocol::CANIC_ROOT_COMMAND,
                &Command::ObserveCanister(FleetCanisterObservabilityRequest {
                    canister_id: request.parent,
                    request: CanisterObservabilityRequest::ChildFunding(request.child),
                }),
            )
            .map_err(crate::fleet_ensure::ops::current_protocol::CurrentProtocolError::from)?;
            let Response::ObserveCanister(CanisterObservabilityResponse::ChildFunding(value)) =
                response
            else {
                return Err(FundingObservationError::AuthorityMismatch.into());
            };
            observation::project_child(value, request.parent, request.child).map_err(unavailable)?
        };
        Ok(FundingChildAccountingRecord {
            native_cycles,
            parent: request.parent,
            child: request.child,
            observed_at_ns: usage.observed_at_ns,
            accounted_cycles: usage.accounted_cycles,
            last_accounted_at_secs: usage.last_accounted_at_secs,
            pending_operations: usage.pending_operations,
            reserved_cycles: usage.reserved_cycles,
        })
    }
}

const fn unavailable(_: StartupUsageUnavailable) -> FundingObservationError {
    FundingObservationError::Unavailable
}

fn require_plan(
    platform: &IcpEnsurePlatform,
    plan: &FleetEnsurePlan,
) -> Result<(), FundingObservationError> {
    if plan.reviewed_desired.as_deref()
        != Some(&ReviewedDesiredFleetRecord::capture(&platform.desired))
    {
        return Err(FundingObservationError::AuthorityMismatch);
    }
    Ok(())
}

fn principal(
    desired: &crate::fleet_ensure::model::DesiredFleet,
    name: &str,
) -> Result<Principal, FundingObservationError> {
    desired
        .canisters
        .iter()
        .find(|entry| entry.name == name)
        .and_then(|entry| entry.principal.as_deref())
        .and_then(|id| id.parse().ok())
        .ok_or(FundingObservationError::AuthorityMismatch)
}

fn qualify_owner(
    platform: &IcpEnsurePlatform,
    desired: &crate::fleet_ensure::model::DesiredFleet,
    name: &str,
    protocol: &ProtocolEntry,
) -> Result<u128, IcpEnsurePlatformError> {
    let id = principal(desired, name)?;
    let observed = platform
        .read_status_optional(&id.to_text())?
        .ok_or(FundingObservationError::Unavailable)?;
    let configured = desired
        .canisters
        .iter()
        .find(|entry| entry.name == name)
        .ok_or(FundingObservationError::AuthorityMismatch)?;
    let mut controllers = configured.controllers.clone();
    for controller in &configured.controller_canisters {
        controllers.push(principal(desired, controller)?.to_text());
    }
    controllers.sort();
    if observed.controllers != controllers
        || observed.status != CanisterRuntimeStatus::Running
        || !observed.module_sha256.as_ref().is_some_and(|hash| {
            hash == &protocol.raw_module_hash || hash == &protocol.installed_module_hash
        })
    {
        return Err(FundingObservationError::AuthorityMismatch.into());
    }
    Ok(observed.cycles)
}

fn catalog(
    platform: &IcpEnsurePlatform,
) -> Result<(AppConfigSnapshot, ProtocolCatalog), IcpEnsurePlatformError> {
    let protocol = platform
        .desired
        .protocol
        .as_ref()
        .ok_or(FundingObservationError::Unavailable)?;
    let bootstrap = platform
        .desired
        .bootstrap
        .as_ref()
        .ok_or(FundingObservationError::Unavailable)?;
    let config_path = platform.root.join(&protocol.app_config);
    let config = AppConfigSnapshot::load(&config_path)
        .map_err(|_| FundingObservationError::AuthorityMismatch)?;
    if config
        .model()
        .compile_component_deployment_configuration()
        .map_err(|_| FundingObservationError::AuthorityMismatch)?
        != bootstrap.component_deployment_configuration
    {
        return Err(FundingObservationError::AuthorityMismatch.into());
    }
    let catalog = ProtocolCatalog::load(
        &platform.root,
        &config_path,
        &config,
        bootstrap.release_build_id,
        &platform.root.join(&protocol.coordinator_candid),
        &platform.root.join(&protocol.root_candid),
        &platform.root.join(&protocol.store_candid),
    )?;
    Ok((config, catalog))
}

fn children(
    platform: &IcpEnsurePlatform,
    desired: &crate::fleet_ensure::model::DesiredFleet,
    root_name: &str,
    root: Principal,
    protocol: &ProtocolEntry,
    registry: &StartupFundingRegistry,
) -> Result<Vec<StartupChildFundingUsage>, IcpEnsurePlatformError> {
    let mut cursor = None;
    let mut seen = BTreeSet::new();
    let mut children = Vec::new();
    loop {
        let page = platform.query_estate_pool_page(&protocol.candid_path, root, cursor)?;
        for entry in page.entries {
            if !seen.insert(entry.canister_id) || seen.len() > MAX_FLEET_ENSURE_CANISTERS {
                return Err(FundingObservationError::AuthorityMismatch.into());
            }
            if !matches!(entry.status, CanisterPoolAssetStatus::Workload { .. }) {
                continue;
            }
            let binding = binding::observe(
                &platform.icp,
                &protocol.candid_path,
                root,
                entry.canister_id,
                &entry.status,
            )
            .map_err(unavailable)?;
            live_binding::current(desired, root_name, &binding, registry).map_err(unavailable)?;
            children.push(StartupChildFundingUsage {
                allowance: Err(StartupUsageUnavailable::NotObserved),
                observed_balance_cycles: None,
                local_demand: Err(StartupDemandUnavailable::BalanceNotObserved),
                binding: Some(binding),
                name: entry.canister_id.to_text(),
                child: entry.canister_id.to_text(),
                usage: Err(StartupUsageUnavailable::NotObserved),
            });
        }
        let Some(next) = page.next_start_after else {
            break;
        };
        if cursor.is_some_and(|prior| next <= prior) || !seen.contains(&next) {
            return Err(FundingObservationError::AuthorityMismatch.into());
        }
        cursor = Some(next);
    }
    Ok(children)
}

fn inspection_response(
    response: Response,
    root: Principal,
    child: Principal,
) -> Result<Box<CanisterStatusResponse>, FundingObservationError> {
    match response {
        Response::InspectCanister(status) => Ok(status),
        Response::InspectionReserveRequired(evidence)
            if evidence.caller == root
                && evidence.canister_id == child
                && evidence.available_liquid_cycles <= evidence.native_cycles =>
        {
            Err(FundingObservationError::Underfunded)
        }
        _ => Err(FundingObservationError::AuthorityMismatch),
    }
}
