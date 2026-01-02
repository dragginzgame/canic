use crate::{
    PublicError,
    cdk::{candid::CandidType, types::Principal},
    dto::rpc::{CreateCanisterParent, CreateCanisterResponse, UpgradeCanisterResponse},
    ids::CanisterRole,
    workflow,
};

///
/// RPC API helpers.
///
/// Public, user-callable wrappers for Canic’s internal RPC workflows.
///
/// These functions:
/// - form part of the **public API surface**
/// - are safe to call from downstream canister `lib.rs` code
/// - return [`PublicError`] suitable for IC boundaries
///
/// Internally, they delegate to workflow-level RPC implementations,
/// preserving the layering:
///
///   user canister -> api -> workflow -> ops -> infra
///
/// Workflow returns internal [`Error`]; conversion to [`PublicError`]
/// happens exclusively at this API boundary.
///

pub async fn create_canister_request<A>(
    canister_role: &CanisterRole,
    parent: CreateCanisterParent,
    extra: Option<A>,
) -> Result<CreateCanisterResponse, PublicError>
where
    A: CandidType + Send + Sync,
{
    workflow::rpc::request::create_canister_request(canister_role, parent, extra)
        .await
        .map_err(PublicError::from)
}

pub async fn upgrade_canister_request(
    canister_pid: Principal,
) -> Result<UpgradeCanisterResponse, PublicError> {
    workflow::rpc::request::upgrade_canister_request(canister_pid)
        .await
        .map_err(PublicError::from)
}
