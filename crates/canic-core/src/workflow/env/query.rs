//! Module: workflow::env::query
//!
//! Responsibility: expose read-only environment workflow snapshots.
//! Does not own: env storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query facade over runtime env ops.

use crate::{dto::env::EnvSnapshotResponse, ops::runtime::env::EnvOps};

///
/// EnvQuery
///

pub struct EnvQuery;

impl EnvQuery {
    #[must_use]
    pub fn snapshot() -> EnvSnapshotResponse {
        EnvOps::snapshot_response()
    }
}
