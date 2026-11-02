//! Module: api::runtime::topup_fixture
//!
//! Responsibility: expose focused IC fixture drivers behind internal-test-fixtures.
//! Does not own: attempt state, funding, replay or failure classification.
//! Boundary: authenticated disposable fixtures delegate to the maintained workflow.

use crate::{
    dto::{error::Error, rpc::CyclesResponse},
    workflow::runtime::cycles::fixture,
};

pub use fixture::{
    CALLBACK_GATE_ID, CALLBACK_GATE_LIMITS, CALLBACK_ROUND_ID, CallbackFault, CallbackProbe,
};

/// Select a trap or held-call boundary in a disposable production callback.
pub fn configure_callback(fault: CallbackFault) {
    fixture::configure_callback(fault);
}

/// Inspect the callback and its actual durable attempt.
#[must_use]
pub fn callback_probe() -> CallbackProbe {
    fixture::callback_probe()
}

/// Serve the authenticated disposable application's real held self-call.
pub async fn hold_callback_reply() {
    fixture::hold_callback_reply().await;
}

/// Drive a real caller-owned request, optionally losing its delivered reply.
pub async fn request(
    cycles: u128,
    discard_reply: bool,
) -> Result<([u8; 32], Option<CyclesResponse>), Error> {
    fixture::request(cycles, discard_reply).await
}

/// Submit a deliberately conflicting identity through the maintained parent RPC.
pub async fn collide(cycles: u128, operation_id: [u8; 32]) -> Result<CyclesResponse, Error> {
    fixture::collide(cycles, operation_id).await
}
