use super::*;
use crate::{
    dto::{
        auth::{RoleAttestation, SignedRoleAttestation},
        capability::{
            CAPABILITY_VERSION_V1, CapabilityProof, DelegatedGrant, DelegatedGrantProof,
            DelegatedGrantScope, PROOF_VERSION_V1, RoleAttestationProof,
        },
        rpc::{CyclesRequest, RootRequestMetadata},
    },
    ops::storage::state::subnet::SubnetStateOps,
};
use k256::ecdsa::{Signature, SigningKey, signature::hazmat::PrehashSigner};

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn sample_request(cycles: u128) -> Request {
    Request::Cycles(CyclesRequest {
        cycles,
        metadata: None,
    })
}

fn sample_metadata(
    request_id: u8,
    nonce: u8,
    issued_at: u64,
    ttl_seconds: u32,
) -> CapabilityRequestMetadata {
    CapabilityRequestMetadata {
        request_id: [request_id; 16],
        nonce: [nonce; 16],
        issued_at,
        ttl_seconds,
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
    let req_a = Request::Cycles(CyclesRequest {
        cycles: 10,
        metadata: Some(RootRequestMetadata {
            request_id: [1u8; 32],
            ttl_seconds: 60,
        }),
    });
    let req_b = Request::Cycles(CyclesRequest {
        cycles: 10,
        metadata: Some(RootRequestMetadata {
            request_id: [2u8; 32],
            ttl_seconds: 120,
        }),
    });

    let hash_a = root_capability_hash(p(1), CAPABILITY_VERSION_V1, &req_a).expect("hash a");
    let hash_b = root_capability_hash(p(1), CAPABILITY_VERSION_V1, &req_b).expect("hash b");
    assert_eq!(hash_a, hash_b);
}

#[test]
fn project_replay_metadata_rejects_expired_metadata() {
    let err = project_replay_metadata(sample_metadata(1, 2, 900, 50), 1_000)
        .expect_err("expired metadata must fail");
    assert!(err.message.contains("expired"));
}

#[test]
fn project_replay_metadata_rejects_future_metadata_beyond_skew() {
    let err = project_replay_metadata(sample_metadata(1, 2, 1_031, 60), 1_000)
        .expect_err("future metadata must fail");
    assert!(err.message.contains("future"));
}

#[test]
fn project_replay_metadata_binds_nonce_into_request_id() {
    let a = project_replay_metadata(sample_metadata(3, 1, 1_000, 60), 1_000).expect("a");
    let b = project_replay_metadata(sample_metadata(3, 2, 1_000, 60), 1_000).expect("b");
    assert_ne!(a.request_id, b.request_id);
}

#[test]
fn with_root_request_metadata_overrides_existing_metadata() {
    let request = Request::Cycles(CyclesRequest {
        cycles: 10,
        metadata: Some(RootRequestMetadata {
            request_id: [7u8; 32],
            ttl_seconds: 10,
        }),
    });
    let metadata = RootRequestMetadata {
        request_id: [9u8; 32],
        ttl_seconds: 60,
    };

    let updated = with_root_request_metadata(request, metadata);
    match updated {
        Request::Cycles(req) => assert_eq!(req.metadata, Some(metadata)),
        _ => panic!("expected cycles request"),
    }
}

fn sample_signed_attestation() -> SignedRoleAttestation {
    SignedRoleAttestation {
        payload: RoleAttestation {
            subject: p(1),
            role: crate::ids::CanisterRole::ROOT,
            subnet_id: None,
            audience: p(2),
            issued_at: 1_000,
            expires_at: 2_000,
            epoch: 1,
        },
        signature: vec![],
        key_id: 1,
    }
}

fn sample_delegated_grant_proof(
    capability: &Request,
    caller: Principal,
    target_canister: Principal,
    now_secs: u64,
) -> DelegatedGrantProof {
    let capability_hash =
        root_capability_hash(target_canister, CAPABILITY_VERSION_V1, capability).expect("hash");
    DelegatedGrantProof {
        proof_version: PROOF_VERSION_V1,
        capability_hash,
        grant: DelegatedGrant {
            issuer: target_canister,
            subject: caller,
            audience: vec![target_canister],
            scope: DelegatedGrantScope {
                service: CapabilityService::Root,
                capability_family: root_capability_family(capability).to_string(),
            },
            capability_hash,
            quota: 1,
            issued_at: now_secs.saturating_sub(10),
            expires_at: now_secs.saturating_add(10),
            epoch: 0,
        },
        grant_sig: vec![1],
        key_id: DELEGATED_GRANT_KEY_ID_V1,
    }
}

fn role_attestation_capability_proof(proof_version: u16) -> CapabilityProof {
    RoleAttestationProof {
        proof_version,
        capability_hash: [0u8; 32],
        attestation: sample_signed_attestation(),
    }
    .try_into()
    .expect("role attestation proof should encode")
}

fn delegated_grant_capability_proof(proof: DelegatedGrantProof) -> CapabilityProof {
    proof
        .try_into()
        .expect("delegated grant proof should encode")
}

fn sign_delegated_grant(seed: u8, grant: &DelegatedGrant) -> (Vec<u8>, Vec<u8>) {
    let signing_key = SigningKey::from_bytes((&[seed; 32]).into()).expect("signing key");
    let signature: Signature = signing_key
        .sign_prehash(&delegated_grant_hash(grant).expect("hash"))
        .expect("prehash signature");
    let public_key = signing_key
        .verifying_key()
        .to_encoded_point(true)
        .as_bytes()
        .to_vec();
    (public_key, signature.to_bytes().to_vec())
}

#[test]
fn role_attestation_blob_round_trips() {
    let proof = RoleAttestationProof {
        proof_version: PROOF_VERSION_V1,
        capability_hash: [7u8; 32],
        attestation: sample_signed_attestation(),
    };

    let blob = super::proof::encode_role_attestation_blob(&proof).expect("encode blob");
    let decoded = super::proof::decode_role_attestation_blob(&blob).expect("decode blob");

    assert_eq!(decoded, proof);
}

#[test]
fn delegated_grant_blob_rejects_header_mismatch() {
    let request = sample_request(10);
    let proof = sample_delegated_grant_proof(&request, p(2), p(1), 100);
    let mut blob = super::proof::encode_delegated_grant_blob(&proof).expect("encode blob");
    blob.capability_hash = [9u8; 32];

    let err =
        super::proof::decode_delegated_grant_blob(&blob).expect_err("header mismatch must fail");
    assert!(err.message.contains("wire header"));
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
fn validate_nonroot_cycles_envelope_rejects_non_structural_proof() {
    let err = validate_nonroot_cycles_envelope(
        CapabilityService::Root,
        CAPABILITY_VERSION_V1,
        &role_attestation_capability_proof(PROOF_VERSION_V1),
    )
    .expect_err("non-root path must reject non-structural proof");
    assert!(err.message.contains("only supports structural proof mode"));
}

#[test]
fn validate_root_capability_envelope_rejects_capability_version_mismatch() {
    let err = validate_root_capability_envelope(
        CapabilityService::Root,
        CAPABILITY_VERSION_V1 + 1,
        &CapabilityProof::Structural,
    )
    .expect_err("unsupported capability version must fail");
    assert!(err.message.contains("capability_version"));
}

#[test]
fn validate_root_capability_envelope_rejects_role_attestation_proof_version_mismatch() {
    let err = validate_root_capability_envelope(
        CapabilityService::Root,
        CAPABILITY_VERSION_V1,
        &role_attestation_capability_proof(PROOF_VERSION_V1 + 1),
    )
    .expect_err("unsupported role proof version must fail");
    assert!(err.message.contains("proof_version"));
}

#[test]
fn validate_root_capability_envelope_rejects_delegated_grant_proof_version_mismatch() {
    let request = sample_request(10);
    let mut proof = sample_delegated_grant_proof(&request, p(2), p(1), 100);
    proof.proof_version = PROOF_VERSION_V1 + 1;

    let err = validate_root_capability_envelope(
        CapabilityService::Root,
        CAPABILITY_VERSION_V1,
        &delegated_grant_capability_proof(proof),
    )
    .expect_err("unsupported delegated grant proof version must fail");
    assert!(err.message.contains("proof_version"));
}

#[test]
fn verify_capability_hash_binding_rejects_mismatch() {
    let err =
        verify_capability_hash_binding(p(1), CAPABILITY_VERSION_V1, &sample_request(10), [0u8; 32])
            .expect_err("mismatched hash must fail");
    assert!(err.message.contains("capability_hash"));
}

#[test]
fn verify_capability_hash_binding_accepts_match() {
    let request = sample_request(10);
    let hash = root_capability_hash(p(1), CAPABILITY_VERSION_V1, &request).expect("hash");
    verify_capability_hash_binding(p(1), CAPABILITY_VERSION_V1, &request, hash)
        .expect("matching hash must verify");
}

#[test]
fn verify_delegated_grant_hash_binding_rejects_mismatch() {
    let proof = DelegatedGrantProof {
        proof_version: PROOF_VERSION_V1,
        capability_hash: [1u8; 32],
        grant: crate::dto::capability::DelegatedGrant {
            issuer: p(1),
            subject: p(2),
            audience: vec![p(3)],
            scope: crate::dto::capability::DelegatedGrantScope {
                service: CapabilityService::Root,
                capability_family: "root".to_string(),
            },
            capability_hash: [2u8; 32],
            quota: 1,
            issued_at: 1,
            expires_at: 2,
            epoch: 0,
        },
        grant_sig: vec![],
        key_id: 1,
    };

    let err = verify_delegated_grant_hash_binding(&proof)
        .expect_err("mismatched delegated grant hash must fail");
    assert!(err.message.contains("capability_hash"));
}

#[test]
fn delegated_grant_hash_changes_with_payload() {
    let grant_a = DelegatedGrant {
        issuer: p(1),
        subject: p(2),
        audience: vec![p(1)],
        scope: DelegatedGrantScope {
            service: CapabilityService::Root,
            capability_family: "RequestCycles".to_string(),
        },
        capability_hash: [1u8; 32],
        quota: 1,
        issued_at: 10,
        expires_at: 20,
        epoch: 0,
    };
    let mut grant_b = grant_a.clone();
    grant_b.quota = 2;

    let hash_a = delegated_grant_hash(&grant_a).expect("hash a");
    let hash_b = delegated_grant_hash(&grant_b).expect("hash b");
    assert_ne!(hash_a, hash_b);
}

#[test]
fn verify_root_delegated_grant_claims_accepts_matching_scope() {
    let now_secs = 100;
    let caller = p(2);
    let target_canister = p(1);
    let capability = sample_request(10);
    let proof = sample_delegated_grant_proof(&capability, caller, target_canister, now_secs);

    verify_root_delegated_grant_claims(&capability, &proof, caller, target_canister, now_secs)
        .expect("matching delegated grant claims must verify");
}

#[test]
fn verify_root_delegated_grant_claims_rejects_subject_mismatch() {
    let now_secs = 100;
    let caller = p(2);
    let target_canister = p(1);
    let capability = sample_request(10);
    let mut proof = sample_delegated_grant_proof(&capability, caller, target_canister, now_secs);
    proof.grant.subject = p(3);

    let err =
        verify_root_delegated_grant_claims(&capability, &proof, caller, target_canister, now_secs)
            .expect_err("subject mismatch must fail");
    assert!(err.message.contains("subject"));
}

#[test]
fn verify_root_delegated_grant_claims_rejects_issuer_mismatch() {
    let now_secs = 100;
    let caller = p(2);
    let target_canister = p(1);
    let capability = sample_request(10);
    let mut proof = sample_delegated_grant_proof(&capability, caller, target_canister, now_secs);
    proof.grant.issuer = p(9);

    let err =
        verify_root_delegated_grant_claims(&capability, &proof, caller, target_canister, now_secs)
            .expect_err("issuer mismatch must fail");
    assert!(err.message.contains("issuer"));
}

#[test]
fn verify_root_delegated_grant_claims_rejects_audience_mismatch() {
    let now_secs = 100;
    let caller = p(2);
    let target_canister = p(1);
    let capability = sample_request(10);
    let mut proof = sample_delegated_grant_proof(&capability, caller, target_canister, now_secs);
    proof.grant.audience = vec![p(99)];

    let err =
        verify_root_delegated_grant_claims(&capability, &proof, caller, target_canister, now_secs)
            .expect_err("audience mismatch must fail");
    assert!(err.message.contains("audience"));
}

#[test]
fn verify_root_delegated_grant_claims_rejects_scope_family_mismatch() {
    let now_secs = 100;
    let caller = p(2);
    let target_canister = p(1);
    let capability = sample_request(10);
    let mut proof = sample_delegated_grant_proof(&capability, caller, target_canister, now_secs);
    proof.grant.scope.capability_family = "Upgrade".to_string();

    let err =
        verify_root_delegated_grant_claims(&capability, &proof, caller, target_canister, now_secs)
            .expect_err("scope family mismatch must fail");
    assert!(err.message.contains("capability_family"));
}

#[test]
fn verify_root_delegated_grant_claims_rejects_zero_quota() {
    let now_secs = 100;
    let caller = p(2);
    let target_canister = p(1);
    let capability = sample_request(10);
    let mut proof = sample_delegated_grant_proof(&capability, caller, target_canister, now_secs);
    proof.grant.quota = 0;

    let err =
        verify_root_delegated_grant_claims(&capability, &proof, caller, target_canister, now_secs)
            .expect_err("zero quota must fail");
    assert!(err.message.contains("quota"));
}

#[test]
fn verify_root_delegated_grant_claims_rejects_not_yet_valid_window() {
    let now_secs = 100;
    let caller = p(2);
    let target_canister = p(1);
    let capability = sample_request(10);
    let mut proof = sample_delegated_grant_proof(&capability, caller, target_canister, now_secs);
    proof.grant.issued_at = now_secs + 10;
    proof.grant.expires_at = now_secs + 20;

    let err =
        verify_root_delegated_grant_claims(&capability, &proof, caller, target_canister, now_secs)
            .expect_err("not-yet-valid grant must fail");
    assert!(err.message.contains("not valid yet"));
}

#[test]
fn verify_root_delegated_grant_claims_rejects_expired_window() {
    let now_secs = 100;
    let caller = p(2);
    let target_canister = p(1);
    let capability = sample_request(10);
    let mut proof = sample_delegated_grant_proof(&capability, caller, target_canister, now_secs);
    proof.grant.issued_at = now_secs - 20;
    proof.grant.expires_at = now_secs - 10;

    let err =
        verify_root_delegated_grant_claims(&capability, &proof, caller, target_canister, now_secs)
            .expect_err("expired grant must fail");
    assert!(err.message.contains("expired"));
}

#[test]
fn verify_root_delegated_grant_claims_rejects_key_id_mismatch() {
    let now_secs = 100;
    let caller = p(2);
    let target_canister = p(1);
    let capability = sample_request(10);
    let mut proof = sample_delegated_grant_proof(&capability, caller, target_canister, now_secs);
    proof.key_id = DELEGATED_GRANT_KEY_ID_V1 + 1;

    let err =
        verify_root_delegated_grant_claims(&capability, &proof, caller, target_canister, now_secs)
            .expect_err("unsupported key_id must fail");
    assert!(err.message.contains("key_id"));
}

#[test]
fn verify_root_delegated_grant_signature_accepts_valid_signature() {
    let capability = sample_request(10);
    let proof = sample_delegated_grant_proof(&capability, p(2), p(1), 100);
    let (public_key, signature) = sign_delegated_grant(7, &proof.grant);
    SubnetStateOps::set_delegated_root_public_key("test_key_1".to_string(), public_key);

    verify_root_delegated_grant_signature(&proof.grant, &signature)
        .expect("valid delegated grant signature must verify");
}

#[test]
fn verify_root_delegated_grant_signature_rejects_invalid_signature() {
    let capability = sample_request(10);
    let proof = sample_delegated_grant_proof(&capability, p(2), p(1), 100);
    let (public_key, _signature) = sign_delegated_grant(7, &proof.grant);
    let (_, wrong_signature) = sign_delegated_grant(8, &proof.grant);
    SubnetStateOps::set_delegated_root_public_key("test_key_1".to_string(), public_key);

    let err = verify_root_delegated_grant_signature(&proof.grant, &wrong_signature)
        .expect_err("invalid signature must fail");
    assert!(err.message.contains("signature invalid"));
}
