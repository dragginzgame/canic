//! Module: ops::rpc::request::conversion
//!
//! Responsibility: convert passive root RPC request DTOs for transport and hashing.
//! Does not own: capability authorization, replay policy, or request execution.
//! Boundary: workflow and RPC ops use this single mechanical wire-shape adapter.

use crate::dto::rpc::{
    AcknowledgePlacementReceiptRequest, CreateCanisterRequest, CyclesRequest,
    RecycleCanisterRequest, Request, RootRequestMetadata, UpgradeCanisterRequest,
};

/// Replay metadata extracted from a boundary request before capability projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::ops::rpc) struct CapabilitySourceMetadata {
    pub(in crate::ops::rpc) request_id: [u8; 32],
    pub(in crate::ops::rpc) ttl_ns: u64,
}

/// Mechanical conversion helpers for the passive root request DTO.
pub(in crate::ops::rpc) struct RequestConversionOps;

impl RequestConversionOps {
    /// Return the stable diagnostic label for one boundary variant.
    #[must_use]
    pub(in crate::ops::rpc) const fn diagnostic_variant_label(request: &Request) -> &'static str {
        match request {
            Request::AcknowledgePlacementReceipt(_) => "AcknowledgePlacementReceipt",
            Request::AllocatePlacementChild(_) => "AllocatePlacementChild",
            Request::CreateCanister(_) => "Provision",
            Request::UpgradeCanister(_) => "Upgrade",
            Request::RecycleCanister(_) => "RecycleCanister",
            Request::Cycles(_) => "RequestCycles",
        }
    }

    /// Extract replay metadata without assigning it policy meaning.
    #[must_use]
    pub(in crate::ops::rpc) const fn source_metadata(
        request: &Request,
    ) -> Option<CapabilitySourceMetadata> {
        let metadata = match request {
            Request::AcknowledgePlacementReceipt(request) => request.metadata,
            Request::AllocatePlacementChild(request) | Request::CreateCanister(request) => {
                request.metadata
            }
            Request::UpgradeCanister(request) => request.metadata,
            Request::RecycleCanister(request) => request.metadata,
            Request::Cycles(request) => request.metadata,
        };

        match metadata {
            Some(RootRequestMetadata { request_id, ttl_ns }) => {
                Some(CapabilitySourceMetadata { request_id, ttl_ns })
            }
            None => None,
        }
    }

    /// Project the signed capability payload without replay-only metadata.
    #[must_use]
    pub(in crate::ops::rpc) fn canonical_capability_payload(request: &Request) -> Request {
        match request {
            Request::AcknowledgePlacementReceipt(request) => {
                Request::AcknowledgePlacementReceipt(AcknowledgePlacementReceiptRequest {
                    operation_id: request.operation_id,
                    metadata: None,
                })
            }
            Request::AllocatePlacementChild(request) => {
                Request::AllocatePlacementChild(CreateCanisterRequest {
                    canister_role: request.canister_role.clone(),
                    parent: request.parent.clone(),
                    extra_arg: request.extra_arg.clone(),
                    metadata: None,
                })
            }
            Request::CreateCanister(request) => Request::CreateCanister(CreateCanisterRequest {
                canister_role: request.canister_role.clone(),
                parent: request.parent.clone(),
                extra_arg: request.extra_arg.clone(),
                metadata: None,
            }),
            Request::UpgradeCanister(request) => Request::UpgradeCanister(UpgradeCanisterRequest {
                canister_pid: request.canister_pid,
                metadata: None,
            }),
            Request::RecycleCanister(request) => Request::RecycleCanister(RecycleCanisterRequest {
                canister_pid: request.canister_pid,
                metadata: None,
            }),
            Request::Cycles(request) => Request::Cycles(CyclesRequest {
                cycles: request.cycles,
                metadata: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cdk::types::Principal, ids::CanisterRole};

    fn metadata(id: u8) -> RootRequestMetadata {
        RootRequestMetadata {
            request_id: [id; 32],
            ttl_ns: u64::from(id) + 1,
        }
    }

    #[test]
    fn source_metadata_covers_every_request_variant() {
        let expected = metadata(7);
        let requests = [
            Request::AcknowledgePlacementReceipt(AcknowledgePlacementReceiptRequest {
                operation_id: [1; 32],
                metadata: Some(expected),
            }),
            Request::AllocatePlacementChild(CreateCanisterRequest {
                canister_role: CanisterRole::new("placement"),
                parent: crate::dto::rpc::CreateCanisterParent::ThisCanister,
                extra_arg: None,
                metadata: Some(expected),
            }),
            Request::CreateCanister(CreateCanisterRequest {
                canister_role: CanisterRole::new("app"),
                parent: crate::dto::rpc::CreateCanisterParent::Root,
                extra_arg: None,
                metadata: Some(expected),
            }),
            Request::UpgradeCanister(UpgradeCanisterRequest {
                canister_pid: Principal::from_slice(&[2; 29]),
                metadata: Some(expected),
            }),
            Request::RecycleCanister(RecycleCanisterRequest {
                canister_pid: Principal::from_slice(&[3; 29]),
                metadata: Some(expected),
            }),
            Request::Cycles(CyclesRequest {
                cycles: 100,
                metadata: Some(expected),
            }),
        ];

        for request in requests {
            assert_eq!(
                RequestConversionOps::source_metadata(&request),
                Some(CapabilitySourceMetadata {
                    request_id: expected.request_id,
                    ttl_ns: expected.ttl_ns,
                })
            );
        }
    }

    #[test]
    fn canonical_capability_payload_strips_metadata_from_every_variant() {
        let requests = [
            Request::AcknowledgePlacementReceipt(AcknowledgePlacementReceiptRequest {
                operation_id: [1; 32],
                metadata: Some(metadata(1)),
            }),
            Request::AllocatePlacementChild(CreateCanisterRequest {
                canister_role: CanisterRole::new("placement"),
                parent: crate::dto::rpc::CreateCanisterParent::ThisCanister,
                extra_arg: Some(vec![1, 2, 3]),
                metadata: Some(metadata(1)),
            }),
            Request::CreateCanister(CreateCanisterRequest {
                canister_role: CanisterRole::new("app"),
                parent: crate::dto::rpc::CreateCanisterParent::Root,
                extra_arg: Some(vec![1, 2, 3]),
                metadata: Some(metadata(1)),
            }),
            Request::UpgradeCanister(UpgradeCanisterRequest {
                canister_pid: Principal::from_slice(&[2; 29]),
                metadata: Some(metadata(2)),
            }),
            Request::RecycleCanister(RecycleCanisterRequest {
                canister_pid: Principal::from_slice(&[3; 29]),
                metadata: Some(metadata(3)),
            }),
            Request::Cycles(CyclesRequest {
                cycles: 100,
                metadata: Some(metadata(4)),
            }),
        ];

        for request in requests {
            let canonical = RequestConversionOps::canonical_capability_payload(&request);
            assert_eq!(RequestConversionOps::source_metadata(&canonical), None);
            assert_eq!(
                RequestConversionOps::diagnostic_variant_label(&request),
                RequestConversionOps::diagnostic_variant_label(&canonical)
            );
        }
    }
}
