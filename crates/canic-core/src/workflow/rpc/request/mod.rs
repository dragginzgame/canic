//! Module: workflow::rpc::request
//!
//! Responsibility: expose workflow entry points for root RPC request creation.
//! Does not own: endpoint authentication, request execution, or storage mutation.
//! Boundary: delegates request construction and outbound calls to RPC ops.

pub mod handler;

use crate::{
    InternalError,
    cdk::candid::CandidType,
    dto::rpc::{CreateCanisterParent, CreateCanisterResponse, CyclesResponse},
    ids::CanisterRole,
    ops::rpc::request::RequestOps,
};

///
/// RpcRequestWorkflow
///
/// Workflow facade for creating root-bound RPC requests.
///

pub struct RpcRequestWorkflow;

impl RpcRequestWorkflow {
    /// Create a child canister request through the configured RPC request ops.
    pub async fn create_canister_request<A>(
        canister_role: &CanisterRole,
        parent: CreateCanisterParent,
        extra: Option<A>,
    ) -> Result<CreateCanisterResponse, InternalError>
    where
        A: CandidType + Send + Sync,
    {
        RequestOps::create_canister(canister_role, parent, extra).await
    }

    /// Create a cycles funding request for the current canister context.
    pub async fn request_cycles(cycles: u128) -> Result<CyclesResponse, InternalError> {
        RequestOps::request_cycles(cycles).await
    }
}
