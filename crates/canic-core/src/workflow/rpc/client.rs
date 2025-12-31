use crate::{
    PublicError,
    cdk::candid::CandidType,
    dto::rpc::{CreateCanisterParent, CreateCanisterResponse},
    ids::CanisterRole,
    ops::rpc,
};

///
/// RPC client helpers.
///
/// Workflow wrappers around ops-level RPC calls so endpoints can depend on
/// workflow instead of ops directly.
///

#[allow(dead_code)]
pub async fn create_canister_request<A>(
    canister_role: &CanisterRole,
    parent: CreateCanisterParent,
    extra: Option<A>,
) -> Result<CreateCanisterResponse, PublicError>
where
    A: CandidType + Send + Sync,
{
    rpc::create_canister_request_internal(canister_role, parent, extra)
        .await
        .map_err(PublicError::from)
}
