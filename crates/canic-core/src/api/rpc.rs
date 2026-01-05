use crate::{
    PublicError,
    cdk::{candid::CandidType, types::Principal},
    dto::rpc::{
        CreateCanisterParent, CreateCanisterResponse, Request, Response, UpgradeCanisterResponse,
    },
    ids::CanisterRole,
    workflow::rpc::request::{RpcRequestWorkflow, handler::RootResponseWorkflow},
};

///
/// RpcApi
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

pub struct RpcApi;

impl RpcApi {
    pub async fn create_canister_request<A>(
        canister_role: &CanisterRole,
        parent: CreateCanisterParent,
        extra: Option<A>,
    ) -> Result<CreateCanisterResponse, PublicError>
    where
        A: CandidType + Send + Sync,
    {
        RpcRequestWorkflow::create_canister_request(canister_role, parent, extra)
            .await
            .map_err(PublicError::from)
    }

    pub async fn upgrade_canister_request(
        canister_pid: Principal,
    ) -> Result<UpgradeCanisterResponse, PublicError> {
        RpcRequestWorkflow::upgrade_canister_request(canister_pid)
            .await
            .map_err(PublicError::from)
    }

    pub async fn response(request: Request) -> Result<Response, PublicError> {
        RootResponseWorkflow::response(request)
            .await
            .map_err(PublicError::from)
    }
}
