//! Module: blob_service
//!
//! Responsibility: compose the independent service with Canic lifecycle and endpoints.
//! Does not own: blob semantics, provider policy or a second memory runtime.
//! Boundary: the owning artifact opts into all endpoints and lifecycle participants.

mod endpoints;
mod lifecycle;
mod ops;
mod workflow;

// Candid collects signatures across modules; its final declaration pass needs
// the same imported type names as the endpoint module.
#[cfg(canic_export_candid)]
use endpoints::*;

canic::start!(
    argument_limits = lifecycle::ENVELOPE_LIMITS,
    lifecycle_participant(init = lifecycle::install, post_upgrade = workflow::restore),
);

#[expect(clippy::unused_async, reason = "Canic deferred lifecycle signature")]
async fn canic_setup() {}
#[expect(
    clippy::unused_async,
    reason = "installation is synchronous in the lifecycle participant"
)]
async fn canic_install(_: Option<Vec<u8>>) {}
#[expect(
    clippy::unused_async,
    reason = "restoration is synchronous in the lifecycle participant"
)]
async fn canic_upgrade() {}

canic::finish!();
