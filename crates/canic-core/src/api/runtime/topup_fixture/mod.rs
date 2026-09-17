//! Module: api::runtime::topup_fixture
//!
//! Responsibility: expose focused IC fixture drivers behind internal-test-fixtures.
//! Does not own: attempt state, funding, replay or failure classification.
//! Boundary: authenticated disposable fixtures delegate to the maintained workflow.

use crate::{
    dto::{error::Error, rpc::CyclesResponse},
    workflow::runtime::cycles::fixture,
};

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
