use crate::{
    Error, PublicError,
    cdk::{candid::CandidType, types::Principal},
    dto::rpc::{CreateCanisterParent, CreateCanisterResponse, UpgradeCanisterResponse},
    ids::CanisterRole,
    ops::rpc,
};

///
/// RPC client helpers.
///
/// Workflow wrappers around ops-level RPC calls so endpoints can depend on
/// workflow instead of ops directly.
///

pub(crate) async fn create_canister_request<A>(
    canister_role: &CanisterRole,
    parent: CreateCanisterParent,
    extra: Option<A>,
) -> Result<CreateCanisterResponse, Error>
where
    A: CandidType + Send + Sync,
{
    rpc::create_canister_request(canister_role, parent, extra).await
}

pub(crate) async fn upgrade_canister_request(
    canister_pid: Principal,
) -> Result<UpgradeCanisterResponse, Error> {
    rpc::upgrade_canister_request(canister_pid).await
}
