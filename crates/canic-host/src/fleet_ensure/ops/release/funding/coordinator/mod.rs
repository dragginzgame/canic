//! Reuse the Coordinator's bounded funding status while binding every Root to the reviewed Registry.

use super::{QUERY_DEADLINE, ReleaseFundingError, ReleaseFundingStage, decode_response};
use candid::{CandidType, Principal};
use canic_control_plane::dto::fleet_coordinator::CoordinatorFundingStatusResponse;
use canic_core::{
    dto::fleet_registry::FleetRegistry, protocol,
    shared_support::fleet_funding_policy::fleet_subnet_root_funding_policy_hash,
};
use ic_agent::Agent;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(CandidType)]
enum Request {
    Funding,
}

#[derive(CandidType, Deserialize)]
enum Response {
    Funding(Box<CoordinatorFundingStatusResponse>),
}

pub(super) async fn collect(
    agent: &Agent,
    coordinator: Principal,
    registry: &FleetRegistry,
    remaining_bytes: &mut usize,
) -> Result<CoordinatorFundingStatusResponse, ReleaseFundingError> {
    let fail = |stage| ReleaseFundingError::Observation {
        root: coordinator,
        stage,
    };
    let argument =
        candid::encode_one(Request::Funding).map_err(|_| fail(ReleaseFundingStage::Query))?;
    let bytes = tokio::time::timeout(
        QUERY_DEADLINE,
        agent
            .query(&coordinator, protocol::CANIC_OBSERVABILITY)
            .with_arg(argument)
            .call(),
    )
    .await
    .map_err(|_| fail(ReleaseFundingStage::Query))?
    .map_err(|_| fail(ReleaseFundingStage::Query))?;
    let Response::Funding(status) = decode_response(coordinator, &bytes, remaining_bytes)?;
    validate(&status, registry).map_err(fail)?;
    Ok(*status)
}

pub(super) fn validate(
    status: &CoordinatorFundingStatusResponse,
    registry: &FleetRegistry,
) -> Result<(), ReleaseFundingStage> {
    if status.coordinator != registry.authority.binding.coordinator
        || status.roots.len() != registry.fleet_subnet_roots.len()
    {
        return Err(ReleaseFundingStage::Binding);
    }
    let mut seen = BTreeSet::new();
    for root in &status.roots {
        let Some(expected) = registry
            .fleet_subnet_roots
            .iter()
            .find(|entry| entry.fleet_subnet_root == root.fleet_subnet_root)
        else {
            return Err(ReleaseFundingStage::Binding);
        };
        let policy_matches = root.policy_hash
            == fleet_subnet_root_funding_policy_hash(&expected.funding)
            && root.policy == expected.funding.root_funding;
        if !seen.insert(root.fleet_subnet_root)
            || !policy_matches
            || root.lifecycle_status != expected.status
        {
            return Err(ReleaseFundingStage::Binding);
        }
    }
    Ok(())
}
