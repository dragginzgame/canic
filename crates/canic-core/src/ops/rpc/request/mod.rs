//! Module: ops::rpc::request
//!
//! Responsibility: expose typed request RPC commands and dispatch errors.
//! Does not own: workflow authorization, endpoint handling, or stable state.
//! Boundary: exposes ops-level dispatch helpers and errors.

mod conversion;
mod dispatch;
mod error;

pub(super) use conversion::RequestConversionOps;
pub use dispatch::RequestOps;
pub(super) use error::RequestOpsError;
