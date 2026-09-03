//! Module: observability
//!
//! Responsibility: route exact Fleet telemetry through controller-authenticated Canic surfaces.
//! Does not own: Fleet discovery, metric interpretation, or report rendering.
//! Boundary: Root observations query the Root directly; descendants use its protected relay.

use crate::{
    CanisterProtocolError, call_canister_with_arg,
    fleet_ensure::CurrentFleetResolution,
    icp::IcpCli,
    protocol_binding::{ProtocolBindingError, resolve_registry_protocol_binding},
    query_canister_with_arg,
    registry::RegistryEntry,
};
use candid::{CandidType, Deserialize, Principal, types::principal::PrincipalError};
use canic_core::{
    dto::{
        canister::{CanisterInspectionRequest, CanisterStatusResponse},
        observability::{
            CanisterObservabilityRequest, CanisterObservabilityResponse,
            FleetCanisterObservabilityRequest,
        },
        page::Page,
    },
    ids::CanisterRole,
    protocol,
};
use std::{collections::BTreeSet, path::Path};
use thiserror::Error as ThisError;

#[derive(CandidType)]
enum RootCommandFragment {
    InspectCanister(CanisterInspectionRequest),
    ObserveCanister(FleetCanisterObservabilityRequest),
}

#[derive(CandidType, Deserialize)]
enum RootCommandResponseFragment {
    InspectCanister(Box<CanisterStatusResponse>),
    ObserveCanister(CanisterObservabilityResponse),
}

#[derive(CandidType)]
enum RootStatusRequestFragment {
    CycleBalance,
    CycleHistory(canic_core::dto::page::PageRequest),
    Metrics(canic_core::dto::role::MetricsStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum RootStatusResponseFragment {
    CycleBalance(canic_core::dto::role::CycleBalanceStatusResponse),
    CycleHistory(Page<canic_core::dto::cycles::CycleTrackerEntry>),
    Metrics(Page<canic_core::dto::metrics::MetricEntry>),
}

#[derive(CandidType)]
enum StoreStatusRequestFragment {
    CycleHistory(canic_core::dto::page::PageRequest),
}

#[derive(CandidType, Deserialize)]
enum StoreStatusResponseFragment {
    CycleHistory(Page<canic_core::dto::cycles::CycleTrackerEntry>),
}

/// Failure to route one protected observation through exact current Fleet authority.
#[derive(Debug, ThisError)]
pub enum FleetObservabilityError {
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
    ProtocolBinding(#[from] ProtocolBindingError),

    #[error("Fleet Subnet Root does not expose cycle top-up history")]
    RootCycleTopupsUnsupported,

    #[error("Wasm Store does not expose cycle top-up history")]
    StoreCycleTopupsUnsupported,

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
    environment: &str,
    fleet: &CurrentFleetResolution,
    entry: &RegistryEntry,
    request: CanisterObservabilityRequest,
) -> Result<CanisterObservabilityResponse, FleetObservabilityError> {
    if entry.role.as_deref() == Some(CanisterRole::ROOT.as_str()) {
        return observe_root(icp, icp_root, environment, entry, request);
    }

    let root = fleet_subnet_root_entry(&fleet.registry.entries, entry)?;
    let binding = resolve_registry_protocol_binding(icp_root, environment, root)?;
    let root_canister = parse_principal("Fleet Subnet Root", &root.pid)?;
    let target = parse_principal("observability target", &entry.pid)?;
    if matches!(&request, CanisterObservabilityRequest::CycleBalance) {
        let response: RootCommandResponseFragment = call_canister_with_arg(
            icp,
            &binding,
            root_canister,
            protocol::CANIC_ROOT_COMMAND,
            &RootCommandFragment::InspectCanister(CanisterInspectionRequest {
                canister_id: target,
            }),
        )?;
        let RootCommandResponseFragment::InspectCanister(response) = response else {
            return Err(FleetObservabilityError::UnexpectedRootResponse);
        };
        let cycles = u128::try_from(response.cycles.0).map_err(|_| {
            FleetObservabilityError::CycleBalanceOverflow {
                canister: entry.pid.clone(),
            }
        })?;
        return Ok(CanisterObservabilityResponse::CycleBalance(
            canic_core::dto::role::CycleBalanceStatusResponse { cycles },
        ));
    }
    if entry.role.as_deref() == Some(CanisterRole::WASM_STORE.as_str()) {
        return observe_store(icp, icp_root, environment, entry, request);
    }
    let response: RootCommandResponseFragment = call_canister_with_arg(
        icp,
        &binding,
        root_canister,
        protocol::CANIC_ROOT_COMMAND,
        &RootCommandFragment::ObserveCanister(FleetCanisterObservabilityRequest {
            canister_id: target,
            request,
        }),
    )?;
    let RootCommandResponseFragment::ObserveCanister(response) = response else {
        return Err(FleetObservabilityError::UnexpectedRootResponse);
    };
    Ok(response)
}

fn observe_store(
    icp: &IcpCli,
    icp_root: &Path,
    environment: &str,
    store: &RegistryEntry,
    request: CanisterObservabilityRequest,
) -> Result<CanisterObservabilityResponse, FleetObservabilityError> {
    let request = match request {
        CanisterObservabilityRequest::CycleHistory(page) => {
            StoreStatusRequestFragment::CycleHistory(page)
        }
        CanisterObservabilityRequest::CycleTopups(_) => {
            return Err(FleetObservabilityError::StoreCycleTopupsUnsupported);
        }
        CanisterObservabilityRequest::Metrics(_) => {
            return Err(FleetObservabilityError::StoreMetricsUnsupported);
        }
        CanisterObservabilityRequest::CycleBalance => {
            unreachable!("Store CycleBalance uses Root management inspection");
        }
    };
    let binding = resolve_registry_protocol_binding(icp_root, environment, store)?;
    let store_canister = parse_principal("Wasm Store", &store.pid)?;
    let response: StoreStatusResponseFragment = query_canister_with_arg(
        icp,
        &binding,
        store_canister,
        protocol::CANIC_STATUS,
        &request,
    )?;
    let StoreStatusResponseFragment::CycleHistory(page) = response;
    Ok(CanisterObservabilityResponse::CycleHistory(page))
}

fn observe_root(
    icp: &IcpCli,
    icp_root: &Path,
    environment: &str,
    root: &RegistryEntry,
    request: CanisterObservabilityRequest,
) -> Result<CanisterObservabilityResponse, FleetObservabilityError> {
    let request = match request {
        CanisterObservabilityRequest::CycleBalance => RootStatusRequestFragment::CycleBalance,
        CanisterObservabilityRequest::CycleHistory(page) => {
            RootStatusRequestFragment::CycleHistory(page)
        }
        CanisterObservabilityRequest::CycleTopups(_) => {
            return Err(FleetObservabilityError::RootCycleTopupsUnsupported);
        }
        CanisterObservabilityRequest::Metrics(request) => {
            RootStatusRequestFragment::Metrics(request)
        }
    };
    let binding = resolve_registry_protocol_binding(icp_root, environment, root)?;
    let root_canister = parse_principal("Fleet Subnet Root", &root.pid)?;
    let response: RootStatusResponseFragment = query_canister_with_arg(
        icp,
        &binding,
        root_canister,
        protocol::CANIC_STATUS,
        &request,
    )?;
    Ok(match response {
        RootStatusResponseFragment::CycleBalance(response) => {
            CanisterObservabilityResponse::CycleBalance(response)
        }
        RootStatusResponseFragment::CycleHistory(response) => {
            CanisterObservabilityResponse::CycleHistory(response)
        }
        RootStatusResponseFragment::Metrics(response) => {
            CanisterObservabilityResponse::Metrics(response)
        }
    })
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
