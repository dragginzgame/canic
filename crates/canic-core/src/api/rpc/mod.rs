use crate::{
    cdk::candid::CandidType,
    dto::{
        capability::{
            NonrootCyclesCapabilityEnvelopeV1, NonrootCyclesCapabilityResponseV1,
            RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
        },
        error::Error,
        rpc::{CreateCanisterParent, CreateCanisterResponse, CyclesResponse},
    },
    ids::CanisterRole,
    workflow::rpc::{
        RootCapabilityAuthority, RootCapabilityLifecycleExecutor, capability,
        request::RpcRequestWorkflow,
    },
};

///
/// RpcApi
///
/// Public, user-callable wrappers for Canic's internal RPC workflows.
///
/// These functions:
/// - form part of the public API surface
/// - are safe to call from downstream canister `lib.rs` code
/// - return [`Error`] suitable for IC boundaries
///
/// Internally, they delegate to workflow-level RPC implementations,
/// preserving the layering:
///
///   user canister -> api -> workflow -> ops -> infra
///
/// Workflow preserves typed internal failures; conversion to [`Error`] happens
/// exclusively at this API boundary.
///

pub struct RpcApi;

impl RpcApi {
    /// Dispatch the full root capability envelope verifier/orchestrator path.
    pub async fn response_capability_v1_root(
        envelope: RootCapabilityEnvelopeV1,
        authority: RootCapabilityAuthority,
        lifecycle: &dyn RootCapabilityLifecycleExecutor,
    ) -> Result<RootCapabilityResponseV1, Error> {
        capability::response_capability_v1_root(envelope, authority, lifecycle)
            .await
            .map_err(Error::from)
    }

    /// Dispatch the non-root structural cycles capability path.
    pub async fn response_capability_v1_nonroot(
        envelope: NonrootCyclesCapabilityEnvelopeV1,
    ) -> Result<NonrootCyclesCapabilityResponseV1, Error> {
        capability::response_capability_v1_nonroot(envelope)
            .await
            .map_err(Error::from)
    }

    /// Request one role-admitted direct child through the local Fleet Subnet Root.
    ///
    /// The application must durably allocate a nonzero `operation_id` before
    /// calling and reuse that exact identity after an interrupted or uncertain
    /// result. Reusing the identity with a different request is rejected.
    pub async fn create_canister_request<A>(
        operation_id: [u8; 32],
        canister_role: &CanisterRole,
        parent: CreateCanisterParent,
        extra: Option<A>,
    ) -> Result<CreateCanisterResponse, Error>
    where
        A: CandidType + Send + Sync,
    {
        RpcRequestWorkflow::create_canister_request(operation_id, canister_role, parent, extra)
            .await
            .map_err(Error::from)
    }

    pub async fn request_cycles(cycles: u128) -> Result<CyclesResponse, Error> {
        RpcRequestWorkflow::request_cycles(cycles)
            .await
            .map_err(Error::from)
    }
}
