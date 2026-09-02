//! Module: fleet_ensure::ops::predecessor_root_status
//!
//! Responsibility: query one sealed predecessor Root pool-status projection.
//! Does not own: predecessor selection, release authority, planning, or mutation.
//! Boundary: callers prove the exact live predecessor module before invoking this adapter.

#[cfg(test)]
mod tests;

use crate::{canister_protocol, icp::IcpCli};
use candid::{CandidType, Principal};
use canic_core::{
    dto::pool::{
        CanisterPoolAsset, CanisterPoolCreation, CanisterPoolHandoff, CanisterPoolResponse,
        CanisterPoolStatusRequest,
    },
    ids::FleetSubnetCanisterPoolConfig,
    protocol,
};
use serde::Deserialize;
use thiserror::Error as ThisError;

/// Failure to query or decode the exact accepted predecessor projection.
#[derive(Debug, ThisError)]
pub enum PredecessorRootStatusError {
    #[error("predecessor Root status query failed: {0}")]
    Query(#[from] canister_protocol::CanisterProtocolError),

    #[error("predecessor Root pool response is invalid: {0}")]
    Decode(#[source] candid::Error),

    #[error("predecessor Root pool request was rejected: {0}")]
    Rejected(canic_core::dto::error::Error),
}

#[derive(CandidType)]
enum PredecessorRootStatusRequest {
    Pool(CanisterPoolStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum PredecessorRootStatusResponse {
    Pool(Box<PredecessorCanisterPoolResponse>),
}

/// Query one page from the exact predecessor pool contract.
pub fn query_pool(
    icp: &IcpCli,
    root: Principal,
    start_after: Option<Principal>,
    limit: u16,
) -> Result<CanisterPoolResponse, PredecessorRootStatusError> {
    let bytes = canister_protocol::query_response_bytes(
        icp,
        root,
        protocol::CANIC_STATUS,
        &PredecessorRootStatusRequest::Pool(CanisterPoolStatusRequest { start_after, limit }),
    )?;
    decode_pool_response(&bytes)
}

fn decode_pool_response(bytes: &[u8]) -> Result<CanisterPoolResponse, PredecessorRootStatusError> {
    let response = candid::decode_one::<
        Result<PredecessorRootStatusResponse, canic_core::dto::error::Error>,
    >(bytes)
    .map_err(PredecessorRootStatusError::Decode)?;
    match response {
        Ok(PredecessorRootStatusResponse::Pool(page)) => Ok((*page).into()),
        Err(error) => Err(PredecessorRootStatusError::Rejected(error)),
    }
}

/// Pool response retained by the sealed predecessor Root contract.
#[derive(CandidType, Deserialize)]
struct PredecessorCanisterPoolResponse {
    config: FleetSubnetCanisterPoolConfig,
    tracked: u32,
    store: u32,
    store_deletion_pending: u32,
    pooled: u32,
    workload: u32,
    surplus: u32,
    ready: u32,
    pending_reset: u32,
    claimed: u32,
    recycling: u32,
    handing_off: u32,
    failed: u32,
    completed_handoffs: u64,
    pending_creation: Option<CanisterPoolCreation>,
    pending_handoff: Option<CanisterPoolHandoff>,
    entries: Vec<CanisterPoolAsset>,
    next_start_after: Option<Principal>,
}

impl From<PredecessorCanisterPoolResponse> for CanisterPoolResponse {
    fn from(value: PredecessorCanisterPoolResponse) -> Self {
        Self {
            config: value.config,
            tracked: value.tracked,
            store: value.store,
            store_deletion_pending: value.store_deletion_pending,
            pooled: value.pooled,
            workload: value.workload,
            surplus: value.surplus,
            ready: value.ready,
            pending_reset: value.pending_reset,
            claimed: value.claimed,
            recycling: value.recycling,
            handing_off: value.handing_off,
            failed: value.failed,
            completed_handoffs: value.completed_handoffs,
            pending_creation: value.pending_creation,
            pending_handoff: value.pending_handoff,
            entries: value.entries,
            next_start_after: value.next_start_after,
        }
    }
}

#[cfg(test)]
fn predecessor_from_current(value: &CanisterPoolResponse) -> PredecessorCanisterPoolResponse {
    PredecessorCanisterPoolResponse {
        config: value.config.clone(),
        tracked: value.tracked,
        store: value.store,
        store_deletion_pending: value.store_deletion_pending,
        pooled: value.pooled,
        workload: value.workload,
        surplus: value.surplus,
        ready: value.ready,
        pending_reset: value.pending_reset,
        claimed: value.claimed,
        recycling: value.recycling,
        handing_off: value.handing_off,
        failed: value.failed,
        completed_handoffs: value.completed_handoffs,
        pending_creation: value.pending_creation.clone(),
        pending_handoff: value.pending_handoff.clone(),
        entries: value.entries.clone(),
        next_start_after: value.next_start_after,
    }
}

#[cfg(test)]
pub fn encode_pool_response_fixture(value: &CanisterPoolResponse) -> Vec<u8> {
    let response = PredecessorRootStatusResponse::Pool(Box::new(predecessor_from_current(value)));
    candid::encode_one(Ok::<_, canic_core::dto::error::Error>(response))
        .expect("encode predecessor pool fixture")
}
