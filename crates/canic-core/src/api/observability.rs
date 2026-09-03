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
