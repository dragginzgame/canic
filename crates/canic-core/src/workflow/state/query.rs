//! Module: workflow::state::query
//!
//! Responsibility: expose the read-only app-state workflow snapshot.
//! Does not own: state storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query facade over state storage ops.

use crate::{dto::state::AppStateResponse, ops::storage::state::app::AppStateOps};

///
/// AppStateQuery
///

pub struct AppStateQuery;

impl AppStateQuery {
    #[must_use]
    pub fn snapshot() -> AppStateResponse {
        AppStateOps::snapshot_response()
    }
}
