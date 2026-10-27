//! Module: workflow::runtime::observability
//!
//! Responsibility: relay one exact sensitive observation to a Root-controlled canister.
//! Does not own: ingress authorization, local metric collection, or Fleet topology discovery.
//! Boundary: the target independently authenticates the calling Root as its controller.

use crate::{
    InternalError,
    dto::observability::{CanisterObservabilityRequest, CanisterObservabilityResponse},
    ops::ic::IcOps,
};
use ic_platform::principal::ExternalPrincipal as Principal;

/// Discover canonical accounting without requiring replay links or cleanup indexes.

pub fn release_intents(
    start_after: Option<crate::dto::release_intents::IntentReleaseKey>,
) -> Result<crate::dto::release_intents::IntentReleaseResponse, InternalError> {
    crate::ops::runtime::release_intents::observe(IcOps::canister_self(), start_after)
}

/// Discover exact receipt metadata without filtering expired or completed history.
pub fn release_receipts(
    start_after: Option<[u8; 32]>,
) -> Result<crate::dto::release_receipts::ReplayReleaseResponse, InternalError> {
    crate::ops::runtime::release_receipts::observe(IcOps::canister_self(), start_after)
}

/// Read exact parent-local child funding evidence after controller authentication.
pub fn child_funding(
    child: Principal,
) -> Result<crate::dto::observability::ChildFundingUsage, InternalError> {
    crate::ops::runtime::funding_usage::child(child)
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

    crate::ops::observability_relay::observe(canister_id, request).await
}
