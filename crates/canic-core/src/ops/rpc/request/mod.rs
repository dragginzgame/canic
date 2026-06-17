//! Module: ops::rpc::request
//!
//! Responsibility: expose typed request RPC commands and dispatch errors.
//! Does not own: workflow authorization, endpoint handling, or stable state.
//! Boundary: re-exports request DTOs and ops-level dispatch helpers.

mod dispatch;
mod error;

pub use crate::dto::rpc::{
    CreateCanisterParent, CreateCanisterRequest, CreateCanisterResponse, CyclesRequest,
    CyclesResponse, RecycleCanisterRequest, RecycleCanisterResponse, Request, Response,
    RootRequestMetadata, UpgradeCanisterRequest, UpgradeCanisterResponse,
};
#[expect(unused_imports)]
pub use dispatch::{CyclesRpc, RequestOps, UpgradeCanisterRpc};
pub use error::RequestOpsError;
