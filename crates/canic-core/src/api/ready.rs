//! Module: api::ready
//!
//! Responsibility: public readiness facade for endpoint callers.
//! Does not own: bootstrap state mutation or readiness barrier internals.
//! Boundary: exposes read-only readiness and bootstrap status snapshots.

use crate::{
    dto::state::BootstrapStatusResponse,
    ops::runtime::{bootstrap::BootstrapStatusOps, ready::ReadyOps},
};

///
/// ReadyApi
///
/// Thin endpoint-facing facade for readiness checks.
///

pub struct ReadyApi;

impl ReadyApi {
    /// Return whether Canic runtime invariants have completed restoration.
    #[must_use]
    pub fn is_ready() -> bool {
        ReadyOps::is_ready()
    }

    /// Return the current bootstrap readiness snapshot.
    #[must_use]
    pub fn bootstrap_status() -> BootstrapStatusResponse {
        BootstrapStatusOps::snapshot()
    }
}
