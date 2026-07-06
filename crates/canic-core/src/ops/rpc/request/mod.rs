//! Module: ops::rpc::request
//!
//! Responsibility: expose typed request RPC commands and dispatch errors.
//! Does not own: workflow authorization, endpoint handling, or stable state.
//! Boundary: exposes ops-level dispatch helpers and errors.

mod dispatch;
mod error;

#[expect(unused_imports)]
pub use dispatch::{CyclesRpc, RequestOps, UpgradeCanisterRpc};
pub use error::RequestOpsError;
