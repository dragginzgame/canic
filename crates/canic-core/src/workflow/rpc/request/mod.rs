pub mod handler;

use crate::{
    InternalError,
    dto::rpc::{
        AuthenticatedRequest, AuthenticatedResponse, CreateCanisterParent, CreateCanisterResponse,
        UpgradeCanisterResponse,
    },
    ops::rpc::{RpcOps, request::RequestOps},
    workflow::{prelude::*, rpc::adapter::RpcAdapter},
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
        let parent = RpcAdapter::create_canister_parent_from_dto(parent);
        let response = RequestOps::create_canister(canister_role, parent, extra).await?;

        Ok(RpcAdapter::create_canister_response_to_dto(response))
    }

    pub async fn upgrade_canister_request(
        canister_pid: Principal,
    ) -> Result<UpgradeCanisterResponse, InternalError> {
        let response = RequestOps::upgrade_canister(canister_pid).await?;

        Ok(RpcAdapter::upgrade_canister_response_to_dto(response))
    }

    pub async fn authenticated_request(
        request: AuthenticatedRequest,
    ) -> Result<AuthenticatedResponse, InternalError> {
        RpcOps::call_authenticated_response(request).await
    }
}
