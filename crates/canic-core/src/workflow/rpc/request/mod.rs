pub mod handler;

use crate::{
    Error,
    dto::rpc::{CreateCanisterParent, CreateCanisterResponse, UpgradeCanisterResponse},
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
    ) -> Result<CreateCanisterResponse, Error>
    where
        A: CandidType + Send + Sync,
    {
        RequestOps::create_canister(canister_role, parent, extra).await
    }

    pub async fn upgrade_canister_request(
        canister_pid: Principal,
    ) -> Result<UpgradeCanisterResponse, Error> {
        RequestOps::upgrade_canister(canister_pid).await
    }
}
