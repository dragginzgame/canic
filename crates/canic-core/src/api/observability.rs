//! Module: api::observability
//!
//! Responsibility: expose the Root-owned sensitive-observation relay to generated endpoints.
//! Does not own: caller authorization, target authorization, or metric collection.
//! Boundary: converts relay failures into the public Canic error contract.

use crate::{
    dto::{
        error::Error,
        observability::{CanisterObservabilityRequest, CanisterObservabilityResponse},
    },
    workflow::runtime::observability,
};
use candid::Principal;

/// Public façade for the controller-authenticated Root observability relay.
pub struct ObservabilityApi;

impl ObservabilityApi {
    /// Discover retained replay authority after controller authentication at the endpoint.
    pub fn release_receipts(
        start_after: Option<[u8; 32]>,
    ) -> Result<crate::dto::release_receipts::ReplayReleaseResponse, Error> {
        observability::release_receipts(start_after).map_err(Into::into)
    }

    /// Return child funding usage without changing grants, reservations or replay state.
    pub fn child_funding(
        child: Principal,
    ) -> Result<crate::dto::observability::ChildFundingUsage, Error> {
        observability::child_funding(child).map_err(Into::into)
    }

    /// Observe one canister that independently recognizes this Root as a controller.
    pub async fn observe_root_controlled_canister(
        canister_id: Principal,
        request: CanisterObservabilityRequest,
    ) -> Result<CanisterObservabilityResponse, Error> {
        observability::observe_root_controlled_canister(canister_id, request)
            .await
            .map_err(Into::into)
    }
}
