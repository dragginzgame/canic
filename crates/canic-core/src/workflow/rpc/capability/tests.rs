use super::*;
use crate::dto::{
    capability::{CAPABILITY_VERSION_V1, CapabilityProof},
    error::ErrorCode,
    rpc::{
        AcknowledgePlacementReceiptRequest, CreateCanisterParent, CreateCanisterRequest,
        CyclesRequest, RecycleCanisterRequest, Request, RootRequestMetadata,
        UpgradeCanisterRequest,
    },
};
use crate::ids::CanisterRole;

const NS_PER_SEC: u64 = 1_000_000_000;

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn sample_request(cycles: u128) -> Request {
    Request::Cycles(CyclesRequest {
        cycles,
        metadata: None,
    })
}

fn sample_metadata(request_id: u8, issued_at_ns: u64, ttl_ns: u64) -> CapabilityRequestMetadata {
    CapabilityRequestMetadata {
        request_id: [request_id; 32],
        issued_at_ns,
        ttl_ns,
    }
}

#[test]
fn root_capability_hash_changes_with_payload() {
    let hash_a =
        root_capability_hash(p(1), CAPABILITY_VERSION_V1, &sample_request(10)).expect("hash a");
    let hash_b =
        root_capability_hash(p(1), CAPABILITY_VERSION_V1, &sample_request(11)).expect("hash b");
    assert_ne!(hash_a, hash_b);
}

#[test]
fn root_capability_hash_binds_target_canister() {
    let req = sample_request(10);
    let hash_a = root_capability_hash(p(1), CAPABILITY_VERSION_V1, &req).expect("hash a");
    let hash_b = root_capability_hash(p(2), CAPABILITY_VERSION_V1, &req).expect("hash b");
    assert_ne!(hash_a, hash_b);
}

#[test]
fn root_capability_hash_binds_capability_version() {
    let req = sample_request(10);
    let hash_a = root_capability_hash(p(1), 1, &req).expect("hash a");
    let hash_b = root_capability_hash(p(1), 2, &req).expect("hash b");
    assert_ne!(hash_a, hash_b);
}

#[test]
fn root_capability_hash_ignores_request_metadata() {
    let metadata_a = RootRequestMetadata {
        request_id: [1u8; 32],
        ttl_ns: 60 * NS_PER_SEC,
    };
    let metadata_b = RootRequestMetadata {
        request_id: [2u8; 32],
        ttl_ns: 120 * NS_PER_SEC,
    };
    let pairs = [
        (
            Request::AcknowledgePlacementReceipt(AcknowledgePlacementReceiptRequest {
                operation_id: [9; 32],
                metadata: Some(metadata_a),
            }),
            Request::AcknowledgePlacementReceipt(AcknowledgePlacementReceiptRequest {
                operation_id: [9; 32],
                metadata: Some(metadata_b),
            }),
        ),
        (
            Request::AllocatePlacementChild(CreateCanisterRequest {
                canister_role: CanisterRole::new("placement"),
                parent: CreateCanisterParent::ThisCanister,
                extra_arg: Some(vec![1, 2, 3]),
                metadata: Some(metadata_a),
            }),
            Request::AllocatePlacementChild(CreateCanisterRequest {
                canister_role: CanisterRole::new("placement"),
                parent: CreateCanisterParent::ThisCanister,
                extra_arg: Some(vec![1, 2, 3]),
                metadata: Some(metadata_b),
            }),
        ),
        (
            Request::CreateCanister(CreateCanisterRequest {
                canister_role: CanisterRole::new("app"),
                parent: CreateCanisterParent::Root,
                extra_arg: Some(vec![1, 2, 3]),
                metadata: Some(metadata_a),
            }),
            Request::CreateCanister(CreateCanisterRequest {
                canister_role: CanisterRole::new("app"),
                parent: CreateCanisterParent::Root,
                extra_arg: Some(vec![1, 2, 3]),
                metadata: Some(metadata_b),
            }),
        ),
        (
            Request::UpgradeCanister(UpgradeCanisterRequest {
                canister_pid: p(2),
                metadata: Some(metadata_a),
            }),
            Request::UpgradeCanister(UpgradeCanisterRequest {
                canister_pid: p(2),
                metadata: Some(metadata_b),
            }),
        ),
        (
            Request::RecycleCanister(RecycleCanisterRequest {
                canister_pid: p(3),
                metadata: Some(metadata_a),
            }),
            Request::RecycleCanister(RecycleCanisterRequest {
                canister_pid: p(3),
                metadata: Some(metadata_b),
            }),
        ),
        (
            Request::Cycles(CyclesRequest {
                cycles: 10,
                metadata: Some(metadata_a),
            }),
            Request::Cycles(CyclesRequest {
                cycles: 10,
                metadata: Some(metadata_b),
            }),
        ),
    ];

    for (request_a, request_b) in pairs {
        let hash_a = root_capability_hash(p(1), CAPABILITY_VERSION_V1, &request_a).expect("hash a");
        let hash_b = root_capability_hash(p(1), CAPABILITY_VERSION_V1, &request_b).expect("hash b");
        assert_eq!(hash_a, hash_b);
    }
}

#[test]
fn project_replay_metadata_rejects_expired_metadata() {
    let err = project_replay_metadata(
        sample_metadata(1, 900 * NS_PER_SEC, 50 * NS_PER_SEC),
        1_000 * NS_PER_SEC,
    )
    .expect_err("expired metadata must fail");
    assert_eq!(err.code, ErrorCode::Conflict);
}

#[test]
fn project_replay_metadata_rejects_expiry_boundary() {
    let err = project_replay_metadata(
        sample_metadata(1, 900 * NS_PER_SEC, 50 * NS_PER_SEC),
        950 * NS_PER_SEC,
    )
    .expect_err("metadata at expiry boundary must fail");
    assert_eq!(err.code, ErrorCode::Conflict);
}

#[test]
fn project_replay_metadata_rejects_future_metadata_beyond_skew() {
    let err = project_replay_metadata(
        sample_metadata(1, 1_031 * NS_PER_SEC, 60 * NS_PER_SEC),
        1_000 * NS_PER_SEC,
    )
    .expect_err("future metadata must fail");
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn project_replay_metadata_preserves_durable_request_id() {
    let projected = project_replay_metadata(
        sample_metadata(3, 1_000 * NS_PER_SEC, 60 * NS_PER_SEC),
        1_000 * NS_PER_SEC,
    )
    .expect("metadata must project");
    assert_eq!(projected.request_id, [3; 32]);
}

#[test]
fn with_root_request_metadata_overrides_existing_metadata() {
    let request = Request::Cycles(CyclesRequest {
        cycles: 10,
        metadata: Some(RootRequestMetadata {
            request_id: [7u8; 32],
            ttl_ns: 10 * NS_PER_SEC,
        }),
    });
    let metadata = RootRequestMetadata {
        request_id: [9u8; 32],
        ttl_ns: 60 * NS_PER_SEC,
    };

    let updated = with_root_request_metadata(RootCapability::from_request(request), metadata);
    match updated {
        RootCapability::RequestCycles(req) => assert_eq!(req.metadata, Some(metadata)),
        _ => panic!("expected cycles request"),
    }
}

#[test]
fn validate_nonroot_cycles_envelope_accepts_structural_cycles() {
    validate_nonroot_cycles_envelope(
        CapabilityService::Root,
        CAPABILITY_VERSION_V1,
        &CapabilityProof::Structural,
    )
    .expect("structural cycles envelope must be accepted for non-root path");
}

#[test]
fn validate_root_capability_envelope_rejects_capability_version_mismatch() {
    let err = validate_root_capability_envelope(
        CapabilityService::Root,
        CAPABILITY_VERSION_V1 + 1,
        &CapabilityProof::Structural,
    )
    .expect_err("unsupported capability version must fail");
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn structural_capability_proof_maps_to_the_current_metric_mode() {
    assert!(
        metric_proof_mode(&CapabilityProof::Structural)
            == RootCapabilityMetricProofMode::Structural
    );
}

#[test]
fn verify_capability_hash_binding_rejects_mismatch() {
    let err =
        verify_capability_hash_binding(p(1), CAPABILITY_VERSION_V1, &sample_request(10), [0u8; 32])
            .expect_err("mismatched hash must fail");
    assert_eq!(err.code, ErrorCode::InvalidInput);
}

#[test]
fn verify_capability_hash_binding_accepts_match() {
    let request = sample_request(10);
    let hash = root_capability_hash(p(1), CAPABILITY_VERSION_V1, &request).expect("hash");
    verify_capability_hash_binding(p(1), CAPABILITY_VERSION_V1, &request, hash)
        .expect("matching hash must verify");
}
