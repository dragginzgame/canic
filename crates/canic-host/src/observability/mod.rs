//! Module: observability
//!
//! Responsibility: route exact Fleet telemetry through controller-authenticated Canic surfaces.
//! Does not own: Fleet discovery, metric interpretation, or report rendering.
//! Boundary: Root observations query the Root directly; descendants use its protected relay.

pub(crate) mod relay;

use crate::{
    CanisterProtocolError,
    fleet_ensure::CurrentFleetResolution,
    icp::IcpCli,
    protocol_binding::{ReleaseProtocolBindingError, resolve_release_registry_wire_schema},
    registry::RegistryEntry,
};
use canic_contracts::{
    dto::observability::{CanisterObservabilityRequest, CanisterObservabilityResponse},
    ids::CanisterRole,
    protocol,
};
use ic_platform::principal::ExternalPrincipal as Principal;
use ic_platform::principal::ExternalPrincipalError as PrincipalError;
use std::{collections::BTreeSet, path::Path};
use thiserror::Error as ThisError;

/// Failure to route one protected observation through exact current Fleet authority.

#[derive(Debug, ThisError)]
pub enum FleetObservabilityError {
    #[error(transparent)]
    ProtocolMethod(#[from] crate::protocol_binding::WireArtifactError),

    #[error("portable Auth inspection requires a receiver Component")]
    PortableReceiverUnsupported,

    #[error("Canister {canister} has no owning Fleet Subnet Root in current topology")]
    MissingRoot { canister: String },

    #[error("current Fleet observability topology contains a parent cycle at {canister}")]
    ParentCycle { canister: String },

    #[error("invalid {field} Principal {value}: {source}")]
    Principal {
        field: &'static str,
        value: String,
        #[source]
        source: PrincipalError,
    },

    #[error(transparent)]
    Protocol(#[from] CanisterProtocolError),

    #[error(transparent)]
    ProtocolBinding(#[from] ReleaseProtocolBindingError),

    #[error("terminal Fleet plan has no selected release for protected observations")]
    MissingRelease,

    #[error("Fleet Subnet Root does not expose cycle top-up history")]
    RootCycleTopupsUnsupported,

    #[error("Wasm Store does not maintain child funding grants")]
    StoreChildFundingUnsupported,

    #[error("Wasm Store does not expose cycle top-up history")]
    StoreCycleTopupsUnsupported,

    #[error("Wasm Store does not expose current memory allocations")]
    StoreMemoryAllocationsUnsupported,

    #[error("Wasm Store does not expose runtime metrics")]
    StoreMetricsUnsupported,

    #[error("Fleet Subnet Root returned a different observability response variant")]
    UnexpectedRootResponse,

    #[error("Canister {canister} cycle balance exceeds u128")]
    CycleBalanceOverflow { canister: String },
}

/// Observe one current Fleet canister without granting its human operator lifecycle control.
pub fn observe_fleet_canister(
    icp: &IcpCli,
    icp_root: &Path,
    fleet: &CurrentFleetResolution,
    entry: &RegistryEntry,
    request: CanisterObservabilityRequest,
) -> Result<CanisterObservabilityResponse, FleetObservabilityError> {
    if entry.role.as_deref() == Some(CanisterRole::ROOT.as_str()) {
        return observe_root(icp, entry, request);
    }

    let root = fleet_subnet_root_entry(&fleet.registry.entries, entry)?;
    let release = fleet
        .plan
        .reviewed_desired
        .as_ref()
        .and_then(|reviewed| reviewed.desired().bootstrap.as_ref())
        .ok_or(FleetObservabilityError::MissingRelease)?
        .release_build_id;
    let root_canister = parse_principal("Fleet Subnet Root", &root.pid)?;
    let target = parse_principal("observability target", &entry.pid)?;
    if matches!(&request, CanisterObservabilityRequest::CycleBalance) {
        let _schema = resolve_release_registry_wire_schema(icp_root, release, root)?;
        return icp.measure_canister_inspection(root_canister, target, || {
            crate::canister_protocol::inspection::preflight_inspection(icp, root_canister, target)?;
            let response =
                crate::canister_protocol::inspection::inspect_status(icp, root_canister, target)?;
            let cycles = u128::try_from(response.cycles.0).map_err(|_| {
                FleetObservabilityError::CycleBalanceOverflow {
                    canister: entry.pid.clone(),
                }
            })?;
            Ok(CanisterObservabilityResponse::CycleBalance(
                canic_contracts::dto::role::CycleBalanceStatusResponse { cycles },
            ))
        });
    }
    let schema = resolve_release_registry_wire_schema(icp_root, release, root)?;
    if entry.role.as_deref() == Some(CanisterRole::WASM_STORE.as_str()) {
        return observe_store(icp, entry, request);
    }
    let auth_contract = match &request {
        CanisterObservabilityRequest::PortablePolicy => Some((
            protocol::root_observation::portable_policy::METHOD,
            protocol::root_observation::portable_policy::METHOD_ID,
            protocol::root_observation::portable_policy::REQUEST_LIMITS,
            protocol::root_observation::portable_policy::RESPONSE_LIMITS,
        )),
        CanisterObservabilityRequest::PortableSessions(_) => Some((
            protocol::root_observation::portable_sessions::METHOD,
            protocol::root_observation::portable_sessions::METHOD_ID,
            protocol::root_observation::portable_sessions::REQUEST_LIMITS,
            protocol::root_observation::portable_sessions::RESPONSE_LIMITS,
        )),
        _ => None,
    };
    if let Some((method, id, request_limits, response_limits)) = auth_contract {
        crate::protocol_binding::require_wire_method(
            &schema.schema,
            method,
            id,
            protocol::wire_declaration::MethodKind::Update,
            request_limits,
            response_limits,
        )?;
    }
    let expected_child = child_funding_target(&request);
    let response = relay::observe(icp, &schema, root_canister, target, request)?;
    verify_child_funding_response(response, target, expected_child)
}

fn observe_store(
    icp: &IcpCli,
    store: &RegistryEntry,
    request: CanisterObservabilityRequest,
) -> Result<CanisterObservabilityResponse, FleetObservabilityError> {
    match request {
        CanisterObservabilityRequest::PortablePolicy
        | CanisterObservabilityRequest::PortableSessions(_) => {
            Err(FleetObservabilityError::PortableReceiverUnsupported)
        }
        CanisterObservabilityRequest::CycleHistory(page) => {
            observe_cycle_history(icp, parse_principal("Wasm Store", &store.pid)?, page)
        }
        CanisterObservabilityRequest::ChildFunding(_) => {
            Err(FleetObservabilityError::StoreChildFundingUnsupported)
        }
        CanisterObservabilityRequest::CycleTopups(_) => {
            Err(FleetObservabilityError::StoreCycleTopupsUnsupported)
        }
        CanisterObservabilityRequest::Metrics(_) => {
            Err(FleetObservabilityError::StoreMetricsUnsupported)
        }
        CanisterObservabilityRequest::MemoryAllocations => {
            Err(FleetObservabilityError::StoreMemoryAllocationsUnsupported)
        }
        CanisterObservabilityRequest::CycleBalance => {
            unreachable!("Store CycleBalance uses Root management inspection")
        }
    }
}

fn observe_cycle_history(
    icp: &IcpCli,
    canister: Principal,
    page: canic_contracts::dto::page::PageRequest,
) -> Result<CanisterObservabilityResponse, FleetObservabilityError> {
    let history = crate::canister_protocol::query_wire(
        icp,
        canister,
        protocol::cycle_history::METHOD,
        protocol::cycle_history::METHOD_ID,
        &page,
        protocol::cycle_history::REQUEST_LIMITS,
        protocol::cycle_history::RESPONSE_LIMITS,
    )?;
    Ok(CanisterObservabilityResponse::CycleHistory(history))
}

fn observe_root(
    icp: &IcpCli,
    root: &RegistryEntry,
    request: CanisterObservabilityRequest,
) -> Result<CanisterObservabilityResponse, FleetObservabilityError> {
    match request {
        CanisterObservabilityRequest::PortablePolicy
        | CanisterObservabilityRequest::PortableSessions(_) => {
            Err(FleetObservabilityError::PortableReceiverUnsupported)
        }
        CanisterObservabilityRequest::ChildFunding(child) => {
            let root_canister = parse_principal("Fleet Subnet Root", &root.pid)?;
            let response: canic_contracts::dto::child_funding::ChildFundingUsage =
                crate::canister_protocol::query_wire(
                    icp,
                    root_canister,
                    protocol::child_funding::METHOD,
                    protocol::child_funding::METHOD_ID,
                    &ic_platform::Principal::from(child),
                    protocol::child_funding::REQUEST_LIMITS,
                    protocol::child_funding::RESPONSE_LIMITS,
                )?;
            verify_child_funding_response(
                CanisterObservabilityResponse::ChildFunding(response),
                root_canister,
                Some(child),
            )
        }
        CanisterObservabilityRequest::CycleBalance => {
            let root_canister = parse_principal("Fleet Subnet Root", &root.pid)?;
            let balance = crate::canister_protocol::query_wire(
                icp,
                root_canister,
                protocol::cycle_balance::METHOD,
                protocol::cycle_balance::METHOD_ID,
                &(),
                protocol::cycle_balance::REQUEST_LIMITS,
                protocol::cycle_balance::RESPONSE_LIMITS,
            )?;
            Ok(CanisterObservabilityResponse::CycleBalance(balance))
        }
        CanisterObservabilityRequest::CycleHistory(page) => {
            observe_cycle_history(icp, parse_principal("Fleet Subnet Root", &root.pid)?, page)
        }
        CanisterObservabilityRequest::CycleTopups(_) => {
            Err(FleetObservabilityError::RootCycleTopupsUnsupported)
        }
        CanisterObservabilityRequest::Metrics(request) => {
            let response: canic_contracts::dto::page::Page<
                canic_contracts::dto::metric_page::MetricEntry,
            > = crate::canister_protocol::query_wire(
                icp,
                parse_principal("Fleet Subnet Root", &root.pid)?,
                protocol::protected_metrics::METHOD,
                protocol::protected_metrics::METHOD_ID,
                &request,
                protocol::protected_metrics::REQUEST_LIMITS,
                protocol::protected_metrics::RESPONSE_LIMITS,
            )?;
            Ok(CanisterObservabilityResponse::Metrics(response))
        }
        CanisterObservabilityRequest::MemoryAllocations => {
            let response = crate::canister_protocol::query_wire(
                icp,
                parse_principal("Fleet Subnet Root", &root.pid)?,
                protocol::protected_memory_allocations::METHOD,
                protocol::protected_memory_allocations::METHOD_ID,
                &(),
                protocol::protected_memory_allocations::REQUEST_LIMITS,
                protocol::protected_memory_allocations::RESPONSE_LIMITS,
            )?;
            Ok(CanisterObservabilityResponse::MemoryAllocations(response))
        }
    }
}

const fn child_funding_target(request: &CanisterObservabilityRequest) -> Option<Principal> {
    match request {
        CanisterObservabilityRequest::ChildFunding(child) => Some(*child),
        _ => None,
    }
}

fn verify_child_funding_response(
    response: CanisterObservabilityResponse,
    parent: Principal,
    child: Option<Principal>,
) -> Result<CanisterObservabilityResponse, FleetObservabilityError> {
    if let Some(child) = child {
        let CanisterObservabilityResponse::ChildFunding(value) = &response else {
            return Err(FleetObservabilityError::UnexpectedRootResponse);
        };
        if (value.parent, value.child) != (parent.into(), child.into()) {
            return Err(FleetObservabilityError::UnexpectedRootResponse);
        }
    }
    Ok(response)
}

fn fleet_subnet_root_entry<'a>(
    registry: &'a [RegistryEntry],
    entry: &RegistryEntry,
) -> Result<&'a RegistryEntry, FleetObservabilityError> {
    let mut current = entry;
    let mut visited = BTreeSet::new();
    loop {
        if !visited.insert(current.pid.as_str()) {
            return Err(FleetObservabilityError::ParentCycle {
                canister: current.pid.clone(),
            });
        }
        if current.role.as_deref() == Some(CanisterRole::ROOT.as_str()) {
            return registry
                .iter()
                .find(|candidate| candidate.pid == current.pid)
                .ok_or_else(|| FleetObservabilityError::MissingRoot {
                    canister: entry.pid.clone(),
                });
        }
        let Some(parent) = current.parent_pid.as_deref() else {
            return Err(FleetObservabilityError::MissingRoot {
                canister: entry.pid.clone(),
            });
        };
        current = registry
            .iter()
            .find(|candidate| candidate.pid == parent)
            .ok_or_else(|| FleetObservabilityError::MissingRoot {
                canister: entry.pid.clone(),
            })?;
    }
}

fn parse_principal(field: &'static str, value: &str) -> Result<Principal, FleetObservabilityError> {
    Principal::from_text(value).map_err(|source| FleetObservabilityError::Principal {
        field,
        value: value.to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_funding_response_binds_parent_and_child_for_direct_and_relayed_reads() {
        let parent = Principal::from_slice(&[1]);
        let child = Principal::from_slice(&[2]);
        let usage = canic_contracts::dto::child_funding::ChildFundingUsage {
            parent: parent.into(),
            child: child.into(),
            observed_at_ns: 10,
            accounted_cycles: 0.into(),
            last_accounted_at_secs: 0,
            pending_operations: 0,
            reserved_cycles: Some(0.into()),
        };
        assert!(
            verify_child_funding_response(
                CanisterObservabilityResponse::ChildFunding(usage.clone()),
                parent,
                Some(child)
            )
            .is_ok()
        );
        for (wrong_parent, wrong_child) in [(child, child), (parent, parent)] {
            assert!(matches!(
                verify_child_funding_response(
                    CanisterObservabilityResponse::ChildFunding(usage.clone()),
                    wrong_parent,
                    Some(wrong_child)
                ),
                Err(FleetObservabilityError::UnexpectedRootResponse)
            ));
        }
    }

    #[test]
    fn auth_inspection_never_uses_root_or_store_direct_query_fallbacks() {
        let icp = IcpCli::new("/unused-icp", Some("local".into()));
        let root = entry("aaaaa-aa", "root", None);
        let store = entry("aaaaa-aa", "wasm_store", Some("root"));
        for request in [
            CanisterObservabilityRequest::PortablePolicy,
            CanisterObservabilityRequest::PortableSessions(
                canic_contracts::dto::portable_inspection::Request {
                    after: None,
                    limit: 1,
                },
            ),
        ] {
            assert!(matches!(
                observe_root(&icp, &root, request.clone()),
                Err(FleetObservabilityError::PortableReceiverUnsupported)
            ));
            assert!(matches!(
                observe_store(&icp, &store, request),
                Err(FleetObservabilityError::PortableReceiverUnsupported)
            ));
        }
    }

    fn entry(pid: &str, role: &str, parent: Option<&str>) -> RegistryEntry {
        RegistryEntry {
            pid: pid.to_string(),
            role: Some(role.to_string()),
            parent_pid: parent.map(str::to_string),
            module_hash: None,
            protocol_binding: None,
        }
    }

    #[test]
    fn descendant_observability_resolves_the_exact_owning_root() {
        let coordinator = entry("coordinator", "fleet_coordinator", None);
        let root = entry("root", "root", Some("coordinator"));
        let parent = entry("parent", "hub", Some("root"));
        let child = entry("child", "leaf", Some("parent"));
        let registry = vec![coordinator, root.clone(), parent, child.clone()];

        assert_eq!(
            fleet_subnet_root_entry(&registry, &child)
                .expect("owning Root")
                .pid,
            root.pid
        );
    }
}
