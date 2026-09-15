//! Module: fleet_ensure::ops::startup_funding::observation
//!
//! Responsibility: read current Coordinator grant accounting for a generation preview.
//! Boundary: exact selected code and policy are required; unavailable evidence is never zero usage.

pub mod binding;
#[cfg(test)]
mod tests;

use crate::{
    canister_protocol::query_with_candid,
    fleet_ensure::view::startup_funding::{
        StartupCoordinatorAccounting, StartupCoordinatorUsage, StartupUsageUnavailable,
    },
    icp::IcpCli,
};
use candid::Principal;
use canic_control_plane::dto::fleet_coordinator::{
    CoordinatorFundingStatusResponse, CoordinatorObservabilityRequest,
    CoordinatorObservabilityResponse,
};
use canic_core::{
    control_plane_support::policy::fleet_funding::{
        FleetFundingWindowSnapshot, funding_window_remaining,
    },
    ids::FleetCoordinatorRootFundingPolicy,
};
use std::{collections::BTreeSet, path::Path};

/// Read one exact parent's child ledger; callers bind installed code and the Candid sidecar.
pub fn observe_child(
    icp: &IcpCli,
    candid: &Path,
    parent: Principal,
    child: Principal,
) -> Result<
    crate::fleet_ensure::view::startup_funding::StartupChildAccounting,
    StartupUsageUnavailable,
> {
    use canic_core::dto::observability::{
        CanisterObservabilityRequest, CanisterObservabilityResponse,
    };
    let response: CanisterObservabilityResponse = query_with_candid(
        icp,
        candid,
        parent,
        canic_core::protocol::CANIC_OBSERVABILITY,
        &CanisterObservabilityRequest::ChildFunding(child),
    )
    .map_err(|_| StartupUsageUnavailable::ObservationFailed)?;
    let CanisterObservabilityResponse::ChildFunding(value) = response else {
        return Err(StartupUsageUnavailable::ObservationFailed);
    };
    project_child(value, parent, child)
}

fn project_child(
    value: canic_core::dto::observability::ChildFundingUsage,
    parent: Principal,
    child: Principal,
) -> Result<
    crate::fleet_ensure::view::startup_funding::StartupChildAccounting,
    StartupUsageUnavailable,
> {
    if value.parent != parent || value.child != child {
        return Err(StartupUsageUnavailable::AuthorityMismatch);
    }
    if value.last_accounted_at_secs > value.observed_at_ns / 1_000_000_000
        || (value.pending_operations == 0 && value.reserved_cycles != Some(0.into()))
    {
        return Err(StartupUsageUnavailable::InvalidAccounting);
    }
    Ok(
        crate::fleet_ensure::view::startup_funding::StartupChildAccounting {
            observed_at_ns: value.observed_at_ns,
            accounted_cycles: value.accounted_cycles.to_u128(),
            last_accounted_at_secs: value.last_accounted_at_secs,
            pending_operations: value.pending_operations,
            reserved_cycles: value.reserved_cycles.map(|cycles| cycles.to_u128()),
        },
    )
}

/// Query only the selected current Coordinator through its verified Candid sidecar.
pub fn observe(
    icp: &IcpCli,
    candid: &Path,
    coordinator: Principal,
    roots: &BTreeSet<Principal>,
    policy: &FleetCoordinatorRootFundingPolicy,
) -> StartupCoordinatorUsage {
    let response: Result<CoordinatorObservabilityResponse, _> = query_with_candid(
        icp,
        candid,
        coordinator,
        canic_core::protocol::CANIC_OBSERVABILITY,
        &CoordinatorObservabilityRequest::Funding,
    );
    let Ok(CoordinatorObservabilityResponse::Funding(status)) = response else {
        return StartupCoordinatorUsage::Unavailable(StartupUsageUnavailable::ObservationFailed);
    };
    match project(status, coordinator, roots, policy) {
        Ok(accounting) => StartupCoordinatorUsage::Observed(accounting),
        Err(reason) => StartupCoordinatorUsage::Unavailable(reason),
    }
}

fn project(
    status: CoordinatorFundingStatusResponse,
    coordinator: Principal,
    roots: &BTreeSet<Principal>,
    policy: &FleetCoordinatorRootFundingPolicy,
) -> Result<StartupCoordinatorAccounting, StartupUsageUnavailable> {
    let actual_roots = status
        .roots
        .iter()
        .map(|root| root.fleet_subnet_root)
        .collect::<BTreeSet<_>>();
    if status.coordinator != coordinator
        || actual_roots != *roots
        || actual_roots.len() != status.roots.len()
    {
        return Err(StartupUsageUnavailable::AuthorityMismatch);
    }
    if status.policy_generation == 0
        || status.rotation.is_some()
        || status.policy.as_ref() != Some(policy)
    {
        return Err(StartupUsageUnavailable::PolicyTransition);
    }
    let window = status
        .fleet_window
        .ok_or(StartupUsageUnavailable::InvalidAccounting)?;
    let remaining = funding_window_remaining(
        Some(FleetFundingWindowSnapshot {
            window_start_secs: window.window_start_secs,
            spent_cycles: window.spent_cycles.to_u128(),
            reserved_cycles: window.reserved_cycles.to_u128(),
        }),
        window.window_start_secs,
        policy.budget.maximum_cycles.to_u128(),
    )
    .ok_or(StartupUsageUnavailable::InvalidAccounting)?;
    if status.automatic_grants > policy.maximum_automatic_grants
        || status.automatic_cycles.to_u128() > policy.maximum_automatic_cycles.to_u128()
    {
        return Err(StartupUsageUnavailable::InvalidAccounting);
    }
    Ok(StartupCoordinatorAccounting {
        policy_generation: status.policy_generation,
        funding_enabled: status.funding_enabled,
        window_start_secs: window.window_start_secs,
        spent_cycles: window.spent_cycles.to_u128(),
        reserved_cycles: window.reserved_cycles.to_u128(),
        window_remaining_cycles: remaining,
        automatic_grants: status.automatic_grants,
        maximum_automatic_grants: policy.maximum_automatic_grants,
        automatic_cycles: status.automatic_cycles.to_u128(),
        maximum_automatic_cycles: policy.maximum_automatic_cycles.to_u128(),
        pending_roots: status
            .roots
            .iter()
            .filter(|root| root.current_operation.is_some())
            .count(),
    })
}
