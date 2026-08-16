use super::*;
use crate::dto::{
    capability::{CAPABILITY_VERSION_V1, CapabilityProof},
    rpc::{CyclesRequest, Request, RootRequestMetadata},
};

const NS_PER_SEC: u64 = 1_000_000_000;

fn sample_metadata(request_id: u8, issued_at_ns: u64, ttl_ns: u64) -> CapabilityRequestMetadata {
    CapabilityRequestMetadata {
        request_id: [request_id; 32],
        issued_at_ns,
        ttl_ns,
    }
}

#[test]
fn project_replay_metadata_rejects_expired_metadata() {
    let err = project_replay_metadata(
        sample_metadata(1, 900 * NS_PER_SEC, 50 * NS_PER_SEC),
        1_000 * NS_PER_SEC,
    )
    .expect_err("expired metadata must fail");
    assert_eq!(
        err.code(),
        crate::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
}

#[test]
fn project_replay_metadata_rejects_expiry_boundary() {
    let err = project_replay_metadata(
        sample_metadata(1, 900 * NS_PER_SEC, 50 * NS_PER_SEC),
        950 * NS_PER_SEC,
    )
    .expect_err("metadata at expiry boundary must fail");
    assert_eq!(
        err.code(),
        crate::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
}

#[test]
fn project_replay_metadata_rejects_future_metadata_beyond_skew() {
    let err = project_replay_metadata(
        sample_metadata(1, 1_031 * NS_PER_SEC, 60 * NS_PER_SEC),
        1_000 * NS_PER_SEC,
    )
    .expect_err("future metadata must fail");
    assert_eq!(
        err.code(),
        crate::diagnostics::codes::REQUEST_INVALID.raw_code()
    );
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
    assert_eq!(
        err.code(),
        crate::diagnostics::codes::REQUEST_INVALID.raw_code()
    );
}

#[test]
fn structural_capability_proof_maps_to_the_current_metric_mode() {
    assert!(
        metric_proof_mode(&CapabilityProof::Structural)
            == RootCapabilityMetricProofMode::Structural
    );
}
