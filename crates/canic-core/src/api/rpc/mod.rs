mod capability;

use crate::{
    cdk::{candid::CandidType, types::Principal},
    dto::{
        capability::{
            NonrootCyclesCapabilityEnvelopeV1, NonrootCyclesCapabilityResponseV1,
            RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
        },
        error::Error,
        rpc::{
            CreateCanisterParent, CreateCanisterResponse, CyclesResponse, UpgradeCanisterResponse,
        },
    },
    ids::CanisterRole,
    workflow::rpc::request::RpcRequestWorkflow,
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
/// Workflow returns internal [`InternalError`]; conversion to [`Error`]
/// happens exclusively at this API boundary.
///

pub struct RpcApi;

impl RpcApi {
    /// Dispatch the full root capability envelope verifier/orchestrator path.
    pub async fn response_capability_v1_root(
        envelope: RootCapabilityEnvelopeV1,
    ) -> Result<RootCapabilityResponseV1, Error> {
        capability::response_capability_v1_root(envelope).await
    }

    /// Dispatch the non-root structural cycles capability path.
    pub async fn response_capability_v1_nonroot(
        envelope: NonrootCyclesCapabilityEnvelopeV1,
    ) -> Result<NonrootCyclesCapabilityResponseV1, Error> {
        capability::response_capability_v1_nonroot(envelope).await
    }

    pub async fn create_canister_request<A>(
        canister_role: &CanisterRole,
        parent: CreateCanisterParent,
        extra: Option<A>,
    ) -> Result<CreateCanisterResponse, Error>
    where
        A: CandidType + Send + Sync,
    {
        RpcRequestWorkflow::create_canister_request(canister_role, parent, extra)
            .await
            .map_err(Error::from)
    }

    pub async fn upgrade_canister_request(
        canister_pid: Principal,
    ) -> Result<UpgradeCanisterResponse, Error> {
        RpcRequestWorkflow::upgrade_canister_request(canister_pid)
            .await
            .map_err(Error::from)
    }

    pub async fn request_cycles(cycles: u128) -> Result<CyclesResponse, Error> {
        RpcRequestWorkflow::request_cycles(cycles)
            .await
            .map_err(Error::from)
    }

    pub async fn response_capability_v1(
        envelope: RootCapabilityEnvelopeV1,
    ) -> Result<RootCapabilityResponseV1, Error> {
        capability::response_capability_v1_root(envelope).await
    }
}
