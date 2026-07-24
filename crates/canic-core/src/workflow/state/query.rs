//! Module: workflow::state::query
//!
//! Responsibility: expose the read-only Fleet-state workflow snapshot.
//! Does not own: state storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query facade over state storage ops.

use crate::{dto::state::FleetStateResponse, ops::storage::state::fleet::FleetStateOps};

///
/// FleetStateQuery
///

pub struct FleetStateQuery;

impl FleetStateQuery {
    #[must_use]
    pub fn snapshot() -> FleetStateResponse {
        FleetStateOps::snapshot_response()
    }
}
