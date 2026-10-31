//! Module: ops::rpc
//!
//! Responsibility: perform outbound RPC calls and capability-envelope transport.
//! Does not own: workflow authorization, endpoint DTO definitions, or replay policy.
//! Boundary: wraps IC call ops and preserves wire-level public errors.

pub mod request;
pub mod store_activation;

use crate::{
    InternalError,
    dto::{
        capability::{
            CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
            NonrootCyclesCapabilityEnvelopeV1, RootCapabilityEnvelopeV1,
        },
        error::Error,
        rpc::{CreateCanisterParent, Request, Response},
    },
    ops::{
        ic::{IcOps, call::execute as execute_call},
        prelude::*,
        rpc::request::{RequestConversionOps, RequestOpsError},
        runtime::env::EnvOps,
    },
};
use canic_contracts::protocol::cycles_capability as wire;
use ic_call::{Call, WaitPolicy};
use serde::de::DeserializeOwned;
use thiserror::Error as ThisError;

///
/// RpcOpsError
///
/// Ops-layer failures raised while transporting RPC requests.
///

#[derive(Debug, ThisError)]
pub enum RpcOpsError {
    #[error(transparent)]
    RequestOps(#[from] RequestOpsError),

    // Error is a wire-level contract.
    // It is preserved through the ops boundary.
    #[error("rpc rejected: {0}")]
    RemoteRejected(Error),
}

impl From<RpcOpsError> for InternalError {
    fn from(err: RpcOpsError) -> Self {
        match err {
            RpcOpsError::RemoteRejected(err) => Self::observed_public(err),
            RpcOpsError::RequestOps(err) => err.into(),
        }
    }
}

///
/// Rpc
///
/// Typed RPC command binding a request variant to its response payload.
///

pub trait Rpc {
    type Response: DeserializeOwned;

    fn into_request(self) -> Request;
    fn try_from_response(resp: Response) -> Result<Self::Response, InternalError>;
}

const DEFAULT_CAPABILITY_METADATA_TTL_NS: u64 = 300_000_000_000;

///
/// RpcOps
///
/// Operations-layer facade for executing protocol-level RPC requests.
///

pub struct RpcOps;

impl RpcOps {
    ///
    /// execute_response_rpc
    ///
    /// Executes a protocol-level RPC via Request/Response.
    ///
    pub(crate) async fn execute_response_rpc<R: Rpc>(
        target_pid: Principal,
        rpc: R,
    ) -> Result<R::Response, InternalError> {
        let request = rpc.into_request();
        if !uses_structural_capability_proof(&request) {
            return Err(non_structural_capability_proof_error(&request));
        }

        let call_res = Self::call_response_capability_v1_structural(target_pid, request).await?;
        let response = R::try_from_response(call_res)?;

        Ok(response)
    }

    async fn call_response_capability_v1_structural(
        target_pid: Principal,
        request: Request,
    ) -> Result<Response, InternalError> {
        let root_pid = EnvOps::root_pid()?;
        if target_pid == root_pid {
            let metadata = capability_metadata_from_request(&request);
            let envelope = RootCapabilityEnvelopeV1 {
                service: CapabilityService::Root,
                capability_version: CAPABILITY_VERSION_V1,
                capability: request,
                proof: CapabilityProof::Structural,
                metadata,
            };

            let call = canic_contracts::ops::root_capability::encode_request(envelope)
                .map_err(|_| InternalError::public(crate::diagnostics::codes::CODEC_FAILED))?;
            let bytes = execute_call(
                Call::new(target_pid.into(), call.method, WaitPolicy::Unbounded)
                    .take_raw_args(call.bytes),
            )
            .await?
            .into_bytes();
            let response =
                canic_contracts::ops::root_capability::decode_response(call.method_id, &bytes)
                    .map_err(|_| InternalError::public(crate::diagnostics::codes::CODEC_INVALID))?
                    .map_err(RpcOpsError::RemoteRejected)?;

            return Ok(response.response);
        }

        let metadata = capability_metadata_from_request(&request);
        let Request::Cycles(capability) = request else {
            return Err(InternalError::state_failure());
        };
        let envelope = NonrootCyclesCapabilityEnvelopeV1 {
            service: CapabilityService::Root,
            capability_version: CAPABILITY_VERSION_V1,
            capability,
            proof: CapabilityProof::Structural,
            metadata,
        };

        let bytes =
            ic_wire::native::encode_request(wire::METHOD_ID, &envelope, wire::REQUEST_LIMITS)
                .map_err(|_| InternalError::public(crate::diagnostics::codes::CODEC_FAILED))?;
        let bytes = execute_call(
            Call::new(target_pid.into(), wire::METHOD, WaitPolicy::Unbounded).take_raw_args(bytes),
        )
        .await?
        .into_bytes();
        let response = decode_nonroot_cycles_reply(&bytes)?;

        Ok(Response::Cycles(response.response))
    }
}

fn decode_nonroot_cycles_reply(
    bytes: &[u8],
) -> Result<crate::dto::capability::NonrootCyclesCapabilityResponseV1, InternalError> {
    let result: Result<crate::dto::capability::NonrootCyclesCapabilityResponseV1, Error> =
        ic_wire::native::decode_response(wire::METHOD_ID, bytes, wire::RESPONSE_LIMITS)
            .map_err(|_| InternalError::public(crate::diagnostics::codes::CODEC_INVALID))?;
    result.map_err(|error| RpcOpsError::RemoteRejected(error).into())
}

const fn uses_structural_capability_proof(request: &Request) -> bool {
    match request {
        Request::AllocatePlacementChild(req) | Request::CreateCanister(req) => {
            matches!(&req.parent, CreateCanisterParent::ThisCanister)
        }
        Request::AcknowledgePlacementReceipt(_)
        | Request::RecycleCanister(_)
        | Request::Cycles(_) => true,
    }
}

const fn non_structural_capability_proof_error(_request: &Request) -> InternalError {
    InternalError::state_failure()
}

fn capability_metadata_from_request(request: &Request) -> CapabilityRequestMetadata {
    let metadata = RequestConversionOps::source_metadata(request);
    let request_id = metadata.map_or([0u8; 32], |m| m.request_id);
    let ttl_ns = metadata.map_or(DEFAULT_CAPABILITY_METADATA_TTL_NS, |m| m.ttl_ns);

    CapabilityRequestMetadata {
        request_id,
        issued_at_ns: IcOps::now_nanos(),
        ttl_ns,
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::rpc::{
        AcknowledgePlacementReceiptRequest, CreateCanisterRequest, CyclesRequest,
        RecycleCanisterRequest, RootRequestMetadata,
    };

    #[test]
    fn native_cycles_reply_preserves_rejection_and_codec_error_codes() {
        let rejection: Result<crate::dto::capability::NonrootCyclesCapabilityResponseV1, Error> =
            Err(Error::from_registered(
                crate::diagnostics::codes::SECURITY_CONFLICT,
            ));
        let frame =
            ic_wire::native::encode_response(wire::METHOD_ID, &rejection, wire::RESPONSE_LIMITS)
                .unwrap();
        let error = decode_nonroot_cycles_reply(&frame).unwrap_err();
        assert_eq!(
            Error::from(error).raw_code(),
            crate::diagnostics::codes::SECURITY_CONFLICT
                .raw_code()
                .raw()
        );
        let malformed = decode_nonroot_cycles_reply(&[]).unwrap_err();
        assert_eq!(
            Error::from(malformed).raw_code(),
            crate::diagnostics::codes::CODEC_INVALID.raw_code().raw()
        );
    }

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn capability_metadata_from_request_preserves_request_id_and_ttl_ns() {
        let request_id = std::array::from_fn(|i| u8::try_from(i).unwrap());
        let request = Request::cycles(CyclesRequest {
            cycles: 1,
            metadata: Some(RootRequestMetadata {
                request_id,
                ttl_ns: u64::MAX,
            }),
        });

        let metadata = capability_metadata_from_request(&request);
        assert_eq!(metadata.request_id, request_id);
        assert_eq!(metadata.ttl_ns, u64::MAX);
        assert!(
            metadata.issued_at_ns > 1_700_000_000_000_000_000,
            "issued_at_ns should be host-time nanoseconds in tests"
        );
    }

    #[test]
    fn capability_metadata_from_request_defaults_when_missing() {
        let request = Request::cycles(CyclesRequest {
            cycles: 1,
            metadata: None,
        });

        let metadata = capability_metadata_from_request(&request);
        assert_eq!(metadata.request_id, [0u8; 32]);
        assert_eq!(metadata.ttl_ns, DEFAULT_CAPABILITY_METADATA_TTL_NS);
    }

    #[test]
    fn structural_capability_proof_support_is_exact() {
        assert!(uses_structural_capability_proof(
            &Request::acknowledge_placement_receipt(AcknowledgePlacementReceiptRequest {
                operation_id: [9; 32],
                metadata: None,
            })
        ));
        assert!(uses_structural_capability_proof(&Request::cycles(
            CyclesRequest {
                cycles: 1,
                metadata: None,
            },
        )));
        assert!(uses_structural_capability_proof(
            &Request::recycle_canister(RecycleCanisterRequest {
                allocation_operation_id: [1; 32],
                canister_pid: p(1),
                metadata: None,
            },)
        ));
        assert!(uses_structural_capability_proof(
            &Request::allocate_placement_child(CreateCanisterRequest {
                canister_role: CanisterRole::new("child"),
                parent: CreateCanisterParent::ThisCanister,
                extra_arg: None,
                metadata: None,
            })
        ));
        assert!(uses_structural_capability_proof(&Request::create_canister(
            CreateCanisterRequest {
                canister_role: CanisterRole::new("child"),
                parent: CreateCanisterParent::ThisCanister,
                extra_arg: None,
                metadata: None,
            },
        )));
        assert!(!uses_structural_capability_proof(
            &Request::create_canister(CreateCanisterRequest {
                canister_role: CanisterRole::new("child"),
                parent: CreateCanisterParent::Root,
                extra_arg: None,
                metadata: None,
            },)
        ));
    }
}
