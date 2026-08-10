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
    model::replay::OperationId,
    ops::rpc::request::RequestOps,
};

///
/// RpcRequestWorkflow
///
/// Workflow facade for creating root-bound RPC requests.
///

pub struct RpcRequestWorkflow;

impl RpcRequestWorkflow {
    /// Create an operation-bound child Canister request through the configured RPC ops.
    pub async fn create_canister_request<A>(
        operation_id: [u8; 32],
        canister_role: &CanisterRole,
        parent: CreateCanisterParent,
        extra: Option<A>,
    ) -> Result<CreateCanisterResponse, InternalError>
    where
        A: CandidType + Send + Sync,
    {
        if operation_id == [0; 32] {
            return Err(InternalError::invalid_input(
                "child creation operation ID must be nonzero",
            ));
        }
        RequestOps::create_canister(
            OperationId::from_bytes(operation_id),
            canister_role,
            parent,
            extra,
        )
        .await
    }

    /// Create a cycles funding request for the current canister context.
    pub async fn request_cycles(cycles: u128) -> Result<CyclesResponse, InternalError> {
        RequestOps::request_cycles(cycles).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::error::ErrorCode;
    use futures::executor::block_on;

    #[test]
    fn child_creation_rejects_zero_operation_identity_before_transport() {
        let error = block_on(RpcRequestWorkflow::create_canister_request(
            [0; 32],
            &CanisterRole::new("project_ledger"),
            CreateCanisterParent::ThisCanister,
            Option::<()>::None,
        ))
        .expect_err("zero child creation operation identity must reject");

        assert_eq!(
            error.public_error().map(|error| error.code),
            Some(ErrorCode::InvalidInput)
        );
    }
}
