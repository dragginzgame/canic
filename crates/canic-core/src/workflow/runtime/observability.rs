//! Module: workflow::runtime::observability
//!
//! Responsibility: relay one exact sensitive observation to a Root-controlled canister.
//! Does not own: ingress authorization, local metric collection, or Fleet topology discovery.
//! Boundary: the target independently authenticates the calling Root as its controller.

use crate::{
    InternalError,
    dto::observability::{CanisterObservabilityRequest, CanisterObservabilityResponse},
    ops::{ic::IcOps, rpc::RpcOps},
    protocol,
};
use candid::{CandidType, Principal};
use serde::Deserialize;

#[derive(CandidType)]
enum CanisterCommandFragment {
    Observe(CanisterObservabilityRequest),
}

#[derive(CandidType, Deserialize)]
enum CanisterCommandResponseFragment {
    Observe(CanisterObservabilityResponse),
}

/// Relay protected observability without granting the operator lifecycle control of the target.
pub async fn observe_root_controlled_canister(
    canister_id: Principal,
    request: CanisterObservabilityRequest,
) -> Result<CanisterObservabilityResponse, InternalError> {
    if canister_id == IcOps::canister_self()
        || canister_id == Principal::anonymous()
        || canister_id == Principal::management_canister()
    {
        return Err(InternalError::invalid_input());
    }

    let response: CanisterCommandResponseFragment = RpcOps::call_rpc_result(
        canister_id,
        protocol::CANIC_COMMAND,
        CanisterCommandFragment::Observe(request),
    )
    .await?;
    let CanisterCommandResponseFragment::Observe(response) = response;
    Ok(response)
}
