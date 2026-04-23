pub mod handler;

use crate::{
    InternalError,
    dto::rpc::{
        CreateCanisterParent, CreateCanisterResponse, CyclesResponse, UpgradeCanisterResponse,
    },
    ops::rpc::request::RequestOps,
    workflow::prelude::*,
};

///
/// RpcRequestWorkflow
///

pub struct RpcRequestWorkflow;

impl RpcRequestWorkflow {
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

    pub async fn upgrade_canister_request(
        canister_pid: Principal,
    ) -> Result<UpgradeCanisterResponse, InternalError> {
        RequestOps::upgrade_canister(canister_pid).await
    }

    pub async fn request_cycles(cycles: u128) -> Result<CyclesResponse, InternalError> {
        RequestOps::request_cycles(cycles).await
    }
}
