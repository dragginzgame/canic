// Category C - Artifact / deployment test (embedded static config).
// This test relies on embedded config by design (test stub).

use candid::{Principal, decode_one, encode_args, encode_one, utils::ArgumentEncoder};
use canic_core::dto::{
    auth::{AttestationKeyStatus, RoleAttestationRequest, SignedRoleAttestation},
    capability::{
        CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
        DelegatedGrant, DelegatedGrantProof, DelegatedGrantScope, PROOF_VERSION_V1,
        RoleAttestationProof, RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
    },
    error::{Error, ErrorCode},
    rpc::{CyclesRequest, Request, Response},
    subnet::SubnetIdentity,
};
use canic_core::ids::CanisterRole;
use pocket_ic::PocketIcBuilder;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, MutexGuard, Once},
    time::Duration,
};

const ROOT_INSTALL_CYCLES: u128 = 80_000_000_000_000;
const CANISTER_PACKAGES: [&str; 1] = ["delegation_root_stub"];
const PREBUILT_WASM_DIR_ENV: &str = "CANIC_PREBUILT_WASM_DIR";
static BUILD_ONCE: Once = Once::new();
static PIC_BUILD_SERIAL: Mutex<()> = Mutex::new(());

struct SerialPic {
    pic: pocket_ic::PocketIc,
    _serial_guard: MutexGuard<'static, ()>,
}

impl Deref for SerialPic {
    type Target = pocket_ic::PocketIc;

    fn deref(&self) -> &Self::Target {
        &self.pic
    }
}

impl DerefMut for SerialPic {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.pic
    }
}

#[test]
fn role_attestation_issue_and_verify_happy_path() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");

    let verified: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (issued, 0u64),
    );
    verified.expect("attestation verification failed");
}

#[test]
fn role_attestation_verify_rejects_mismatched_caller() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");

    let verified: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        Principal::anonymous(),
        "root_verify_role_attestation",
        (issued, 0u64),
    );
    let err = verified.expect_err("verification must fail for mismatched caller");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("subject mismatch"),
        "expected subject mismatch error, got: {err:?}"
    );
}

#[test]
fn role_attestation_verify_rejects_expired() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (1u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");

    pic.advance_time(Duration::from_secs(2));
    pic.tick();

    let verified: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (issued, 0u64),
    );
    let err = verified.expect_err("verification must fail for expired attestation");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("expired"),
        "expected expired error, got: {err:?}"
    );
}

#[test]
fn role_attestation_verify_rejects_audience_mismatch() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);
    let wrong_audience = Principal::from_slice(&[9; 29]);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(wrong_audience), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");

    let verified: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (issued, 0u64),
    );
    let err = verified.expect_err("verification must fail for audience mismatch");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("audience mismatch"),
        "expected audience mismatch error, got: {err:?}"
    );
}

#[test]
fn role_attestation_verify_rejects_epoch_floor() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");

    let verified: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (issued, 1u64),
    );
    let err = verified.expect_err("verification must fail when epoch floor is higher");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("epoch"),
        "expected epoch rejection, got: {err:?}"
    );
}

#[test]
fn role_attestation_verify_handles_rotated_key_grace_window() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let previous_key_id = 1_001u32;
    let previous_key_seed = 3u8;
    let current_key_id = 1_002u32;
    let current_key_seed = 4u8;

    let previous_attestation: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test_with_key",
        (
            60u64,
            Some(root_id),
            0u64,
            previous_key_id,
            previous_key_seed,
        ),
    );
    let previous_attestation = previous_attestation.expect("previous-key attestation failed");
    let grace_until = previous_attestation.payload.issued_at.saturating_add(5);

    let set_keys: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_set_test_attestation_key_set",
        (vec![
            (
                previous_key_id,
                previous_key_seed,
                AttestationKeyStatus::Previous,
                None,
                Some(grace_until),
            ),
            (
                current_key_id,
                current_key_seed,
                AttestationKeyStatus::Current,
                Some(previous_attestation.payload.issued_at),
                None,
            ),
        ],),
    );
    set_keys.expect("seed key set failed");

    let verify_previous_in_grace: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (previous_attestation.clone(), 0u64),
    );
    verify_previous_in_grace.expect("previous key should verify during grace");

    pic.advance_time(Duration::from_secs(6));
    pic.tick();

    let verify_previous_after_grace: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (previous_attestation, 0u64),
    );
    let err = verify_previous_after_grace.expect_err("previous key must fail after grace");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("expired"),
        "expected key expiry error, got: {err:?}"
    );

    let current_attestation: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test_with_key",
        (60u64, Some(root_id), 0u64, current_key_id, current_key_seed),
    );
    let current_attestation = current_attestation.expect("current-key attestation failed");

    let verify_current_after_grace: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (current_attestation, 0u64),
    );
    verify_current_after_grace.expect("current key should verify after grace");
}

#[test]
fn capability_endpoint_role_attestation_proof_happy_path() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");
    let issued_at = issued.payload.issued_at;

    let request = Request::Cycles(CyclesRequest {
        cycles: 1,
        metadata: None,
    });
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request.clone(),
        proof: CapabilityProof::RoleAttestation(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: root_capability_hash(root_id, &request),
            attestation: issued,
        }),
        metadata: capability_metadata(issued_at, 1, 9, 60),
    };

    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let response = response.expect("capability endpoint call failed");
    match response.response {
        Response::Cycles(res) => assert_eq!(res.cycles_transferred, 1),
        other => panic!("expected cycles response, got: {other:?}"),
    }
}

#[test]
fn capability_endpoint_rejects_expired_role_attestation() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (1u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");
    let issued_at = issued.payload.issued_at;
    pic.advance_time(Duration::from_secs(2));
    pic.tick();

    let request = Request::Cycles(CyclesRequest {
        cycles: 1,
        metadata: None,
    });
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request.clone(),
        proof: CapabilityProof::RoleAttestation(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: root_capability_hash(root_id, &request),
            attestation: issued,
        }),
        metadata: capability_metadata(issued_at, 2, 8, 60),
    };

    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let err = response.expect_err("expired attestation must fail");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("expired"),
        "expected expired attestation error, got: {err:?}"
    );
}

#[test]
fn capability_endpoint_rejects_audience_mismatch() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);
    let wrong_audience = Principal::from_slice(&[9; 29]);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(wrong_audience), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");
    let issued_at = issued.payload.issued_at;

    let request = Request::Cycles(CyclesRequest {
        cycles: 1,
        metadata: None,
    });
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request.clone(),
        proof: CapabilityProof::RoleAttestation(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: root_capability_hash(root_id, &request),
            attestation: issued,
        }),
        metadata: capability_metadata(issued_at, 3, 7, 60),
    };

    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let err = response.expect_err("audience mismatch must fail");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("audience mismatch"),
        "expected audience mismatch error, got: {err:?}"
    );
}

#[test]
fn capability_endpoint_policy_denies_role_attestation_subject_mismatch() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");
    let issued_at = issued.payload.issued_at;

    let request = Request::IssueRoleAttestation(RoleAttestationRequest {
        subject: Principal::anonymous(),
        role: CanisterRole::ROOT,
        subnet_id: None,
        audience: Some(root_id),
        ttl_secs: 60,
        epoch: 0,
        metadata: None,
    });

    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request.clone(),
        proof: CapabilityProof::RoleAttestation(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: root_capability_hash(root_id, &request),
            attestation: issued,
        }),
        metadata: capability_metadata(issued_at, 4, 6, 60),
    };

    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let err = response.expect_err("policy subject mismatch must fail");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("must match caller"),
        "expected subject mismatch policy error, got: {err:?}"
    );
}

#[test]
fn capability_endpoint_policy_denial_is_not_replay_cached() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");
    let issued_at = issued.payload.issued_at;

    let request = Request::IssueRoleAttestation(RoleAttestationRequest {
        subject: Principal::anonymous(),
        role: CanisterRole::ROOT,
        subnet_id: None,
        audience: Some(root_id),
        ttl_secs: 60,
        epoch: 0,
        metadata: None,
    });

    let envelope_a = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request.clone(),
        proof: CapabilityProof::RoleAttestation(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: root_capability_hash(root_id, &request),
            attestation: issued.clone(),
        }),
        metadata: capability_metadata(issued_at, 4, 66, 60),
    };
    let envelope_b = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request.clone(),
        proof: CapabilityProof::RoleAttestation(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: root_capability_hash(root_id, &request),
            attestation: issued,
        }),
        metadata: capability_metadata(issued_at, 4, 66, 60),
    };

    let first: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope_a,),
    );
    let second: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope_b,),
    );

    let first_err = first.expect_err("first policy denial must fail");
    let second_err = second.expect_err("second policy denial must fail");
    assert_eq!(first_err.code, ErrorCode::Internal);
    assert_eq!(second_err.code, ErrorCode::Internal);
    assert!(
        first_err.message.contains("must match caller"),
        "expected policy denial on first request, got: {first_err:?}"
    );
    assert!(
        second_err.message.contains("must match caller"),
        "expected policy denial on second request, got: {second_err:?}"
    );
    assert!(
        !second_err.message.contains("duplicate replay request"),
        "policy denial should not be replay-cached, got: {second_err:?}"
    );
}

#[test]
fn capability_endpoint_policy_denies_role_attestation_missing_audience() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");
    let issued_at = issued.payload.issued_at;

    let request = Request::IssueRoleAttestation(RoleAttestationRequest {
        subject: root_id,
        role: CanisterRole::ROOT,
        subnet_id: None,
        audience: None,
        ttl_secs: 60,
        epoch: 0,
        metadata: None,
    });

    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request.clone(),
        proof: CapabilityProof::RoleAttestation(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: root_capability_hash(root_id, &request),
            attestation: issued,
        }),
        metadata: capability_metadata(issued_at, 5, 5, 60),
    };

    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let err = response.expect_err("missing audience policy must fail");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("audience is required"),
        "expected audience-required policy error, got: {err:?}"
    );
}

#[test]
fn capability_endpoint_rejects_tampered_signature() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let mut issued = issued.expect("attestation issuance failed");
    let issued_at = issued.payload.issued_at;
    if let Some(first) = issued.signature.first_mut() {
        *first ^= 0x01;
    }

    let request = Request::Cycles(CyclesRequest {
        cycles: 1,
        metadata: None,
    });
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request.clone(),
        proof: CapabilityProof::RoleAttestation(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: root_capability_hash(root_id, &request),
            attestation: issued,
        }),
        metadata: capability_metadata(issued_at, 6, 4, 60),
    };

    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let err = response.expect_err("tampered attestation signature must fail");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("signature"),
        "expected signature error, got: {err:?}"
    );
}

#[test]
fn capability_endpoint_allows_structural_cycles_for_registered_caller() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued_at = issued
        .expect("attestation issuance failed")
        .payload
        .issued_at;

    let request = Request::Cycles(CyclesRequest {
        cycles: 1,
        metadata: None,
    });
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request,
        proof: CapabilityProof::Structural,
        metadata: capability_metadata(issued_at, 7, 3, 60),
    };

    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let response = response.expect("structural cycles proof should succeed");
    match response.response {
        Response::Cycles(res) => assert_eq!(res.cycles_transferred, 1),
        other => panic!("expected cycles response, got: {other:?}"),
    }
}

#[test]
fn capability_endpoint_rejects_structural_for_unsupported_capability() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued_at = issued
        .expect("attestation issuance failed")
        .payload
        .issued_at;

    let request = Request::IssueRoleAttestation(RoleAttestationRequest {
        subject: root_id,
        role: CanisterRole::ROOT,
        subnet_id: None,
        audience: Some(root_id),
        ttl_secs: 60,
        epoch: 0,
        metadata: None,
    });
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request,
        proof: CapabilityProof::Structural,
        metadata: capability_metadata(issued_at, 7, 3, 60),
    };

    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let err = response.expect_err("unsupported structural capability must fail closed");
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert!(
        err.message.contains("only supported"),
        "expected structural capability-scope rejection, got: {err:?}"
    );
}

#[test]
fn capability_endpoint_rejects_delegated_grant_scope_mismatch() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued_at = issued
        .expect("attestation issuance failed")
        .payload
        .issued_at;

    let request = Request::Cycles(CyclesRequest {
        cycles: 1,
        metadata: None,
    });
    let capability_hash = root_capability_hash(root_id, &request);
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request,
        proof: CapabilityProof::DelegatedGrant(DelegatedGrantProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash,
            grant: DelegatedGrant {
                issuer: root_id,
                subject: root_id,
                audience: vec![root_id],
                scope: DelegatedGrantScope {
                    service: CapabilityService::Root,
                    capability_family: "root".to_string(),
                },
                capability_hash,
                quota: 1,
                issued_at,
                expires_at: issued_at.saturating_add(60),
                epoch: 0,
            },
            grant_sig: vec![1, 2, 3],
            key_id: 1,
        }),
        metadata: capability_metadata(issued_at, 8, 2, 60),
    };

    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let err = response.expect_err("delegated grant scope mismatch must fail closed");
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert!(
        err.message.contains("capability_family"),
        "expected delegated-grant scope rejection, got: {err:?}"
    );
}

#[test]
fn capability_endpoint_rejects_capability_hash_mismatch() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");
    let issued_at = issued.payload.issued_at;

    let request = Request::Cycles(CyclesRequest {
        cycles: 1,
        metadata: None,
    });
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request,
        proof: CapabilityProof::RoleAttestation(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: [0u8; 32],
            attestation: issued,
        }),
        metadata: capability_metadata(issued_at, 9, 1, 60),
    };

    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let err = response.expect_err("hash mismatch must fail closed");
    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(
        err.message.contains("capability_hash"),
        "expected capability_hash mismatch error, got: {err:?}"
    );
}

fn install_root_canister(pic: &pocket_ic::PocketIc, wasm: Vec<u8>) -> Principal {
    let root_id = pic.create_canister();
    pic.add_cycles(root_id, ROOT_INSTALL_CYCLES);
    pic.install_canister(
        root_id,
        wasm,
        encode_one(SubnetIdentity::Manual).expect("encode args"),
        None,
    );
    root_id
}

fn update_call_as<T, A>(
    pic: &pocket_ic::PocketIc,
    canister_id: Principal,
    caller: Principal,
    method: &str,
    args: A,
) -> T
where
    T: candid::CandidType + DeserializeOwned,
    A: ArgumentEncoder,
{
    let payload = encode_args(args).expect("encode args");
    let result = pic
        .update_call(canister_id, caller, method, payload)
        .expect("update_call failed");

    decode_one(&result).expect("decode response")
}

fn root_capability_hash(target_canister: Principal, capability: &Request) -> [u8; 32] {
    const CAPABILITY_HASH_DOMAIN_V1: &[u8] = b"CANIC_CAPABILITY_V1";
    let canonical = strip_request_metadata(capability.clone());

    let payload = encode_one(&(
        target_canister,
        CapabilityService::Root,
        CAPABILITY_VERSION_V1,
        canonical,
    ))
    .expect("encode capability payload");
    let mut hasher = Sha256::new();
    hasher.update(CAPABILITY_HASH_DOMAIN_V1);
    hasher.update(payload);
    hasher.finalize().into()
}

fn strip_request_metadata(request: Request) -> Request {
    match request {
        Request::CreateCanister(mut req) => {
            req.metadata = None;
            Request::CreateCanister(req)
        }
        Request::UpgradeCanister(mut req) => {
            req.metadata = None;
            Request::UpgradeCanister(req)
        }
        Request::Cycles(mut req) => {
            req.metadata = None;
            Request::Cycles(req)
        }
        Request::IssueDelegation(mut req) => {
            req.metadata = None;
            Request::IssueDelegation(req)
        }
        Request::IssueRoleAttestation(mut req) => {
            req.metadata = None;
            Request::IssueRoleAttestation(req)
        }
    }
}

const fn capability_metadata(
    issued_at: u64,
    request_id_seed: u8,
    nonce_seed: u8,
    ttl_seconds: u32,
) -> CapabilityRequestMetadata {
    CapabilityRequestMetadata {
        request_id: [request_id_seed; 16],
        nonce: [nonce_seed; 16],
        issued_at,
        ttl_seconds,
    }
}

fn build_canisters_once(workspace_root: &PathBuf) {
    BUILD_ONCE.call_once_force(|_| {
        if prebuilt_wasm_dir().is_some() {
            return;
        }

        let target_dir = test_target_dir(workspace_root);
        let mut cmd = Command::new("cargo");
        cmd.current_dir(workspace_root);
        cmd.env("CARGO_TARGET_DIR", &target_dir);
        cmd.env("DFX_NETWORK", "local");
        cmd.args(["build", "--release", "--target", "wasm32-unknown-unknown"]);
        for name in CANISTER_PACKAGES {
            cmd.args(["-p", name]);
        }

        let output = cmd.output().expect("failed to run cargo build");
        assert!(
            output.status.success(),
            "cargo build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    });
}

// Serialize full PocketIC usage to avoid concurrent server races across tests.
fn build_pic() -> SerialPic {
    let serial_guard = PIC_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    SerialPic {
        pic: PocketIcBuilder::new().with_application_subnet().build(),
        _serial_guard: serial_guard,
    }
}

fn read_wasm(workspace_root: &Path, crate_name: &str) -> Vec<u8> {
    let wasm_path = wasm_path(workspace_root, crate_name);
    fs::read(&wasm_path).unwrap_or_else(|err| panic!("failed to read {crate_name} wasm: {err}"))
}

fn wasm_path(workspace_root: &Path, crate_name: &str) -> PathBuf {
    if let Some(dir) = prebuilt_wasm_dir() {
        return dir.join(format!("{crate_name}.wasm"));
    }

    let target_dir = test_target_dir(workspace_root);

    target_dir
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{crate_name}.wasm"))
}

fn prebuilt_wasm_dir() -> Option<PathBuf> {
    env::var(PREBUILT_WASM_DIR_ENV).ok().map(PathBuf::from)
}

fn test_target_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("target").join("pic-wasm")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}
