pub mod handler;

use crate::{
    Error,
    dto::rpc::{CreateCanisterParent, CreateCanisterResponse, UpgradeCanisterResponse},
    ops,
    workflow::prelude::*,
};

///
/// RPC helpers
///
/// Workflow wrappers around ops-level RPC calls so endpoints can depend on
/// workflow instead of ops directly.
///

pub async fn create_canister_request<A>(
    canister_role: &CanisterRole,
    parent: CreateCanisterParent,
    extra: Option<A>,
) -> Result<CreateCanisterResponse, Error>
where
    A: CandidType + Send + Sync,
{
    ops::rpc::request::create_canister_request(canister_role, parent, extra).await
}

pub async fn upgrade_canister_request(
    canister_pid: Principal,
) -> Result<UpgradeCanisterResponse, Error> {
    ops::rpc::request::upgrade_canister_request(canister_pid).await
}
