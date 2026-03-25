// Category C - Artifact / deployment test (embedded static config).
// This test relies on embedded config by design (test stub).

use candid::{Principal, decode_one, encode_args, encode_one, utils::ArgumentEncoder};
use canic_core::dto::{
    auth::{
        AttestationKeyStatus, DelegatedToken, DelegatedTokenClaims, DelegationAdminCommand,
        DelegationAdminResponse, DelegationProofInstallIntent, DelegationProofInstallRequest,
        DelegationProvisionStatus, DelegationVerifierProofPushRequest, RoleAttestationRequest,
        SignedRoleAttestation,
    },
    capability::{
        CAPABILITY_VERSION_V1, CapabilityProof, CapabilityRequestMetadata, CapabilityService,
        DelegatedGrant, DelegatedGrantProof, DelegatedGrantScope, PROOF_VERSION_V1,
        RoleAttestationProof, RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
    },
    error::{Error, ErrorCode},
    metrics::{MetricsKind, MetricsRequest, MetricsResponse},
    page::PageRequest,
    rpc::{CreateCanisterParent, CreateCanisterRequest},
    rpc::{CyclesRequest, Request, Response},
    subnet::SubnetIdentity,
    topology::SubnetRegistryResponse,
};
use canic_core::ids::{CanisterRole, cap};
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
static BUILD_WITHOUT_TEST_MATERIAL_ONCE: Once = Once::new();
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
#[expect(clippy::too_many_lines)]
fn delegated_session_bootstrap_affects_authenticated_guard_only() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);
    let signer_id = signer_pid(&pic, root_id);
    wait_until_ready(&pic, signer_id);

    let wallet = Principal::from_slice(&[41; 29]);
    let delegated_subject = Principal::from_slice(&[42; 29]);
    let now: Result<u64, Error> =
        query_call_as(&pic, root_id, Principal::anonymous(), "root_now_secs", ());
    let now = now.expect("query root now_secs failed");

    let claims = DelegatedTokenClaims {
        sub: delegated_subject,
        shard_pid: signer_id,
        scopes: vec![cap::VERIFY.to_string()],
        aud: vec![signer_id],
        iat: now,
        exp: now + 120,
    };
    let issued: Result<DelegatedToken, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_test_delegated_token",
        (claims,),
    );
    let token = issued.expect("token issuance failed");

    let keys: Result<(Vec<u8>, Vec<u8>), Error> = query_call_as(
        &pic,
        root_id,
        Principal::anonymous(),
        "root_test_delegation_public_keys",
        (),
    );
    let (root_public_key, shard_public_key) = keys.expect("query test delegation keys failed");
    let install_signer_material: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        root_id,
        "signer_install_test_delegation_material",
        (token.proof.clone(), root_public_key, shard_public_key),
    );
    install_signer_material.expect("install signer delegation material must succeed");

    let denied_before: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_verify_token",
        (token.clone(),),
    );
    let err = denied_before.expect_err("subject mismatch must deny before session bootstrap");
    assert_eq!(err.code, ErrorCode::Unauthorized);
    assert!(
        err.message.contains("does not match caller"),
        "expected subject mismatch denial, got: {err:?}"
    );

    let invalid_bootstrap: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_bootstrap_delegated_session",
        (
            bogus_delegated_token(root_id, signer_id),
            delegated_subject,
            Some(60u64),
        ),
    );
    invalid_bootstrap.expect_err("bogus token bootstrap must fail closed");

    let bootstrap_ok: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_bootstrap_delegated_session",
        (token.clone(), delegated_subject, Some(60u64)),
    );
    bootstrap_ok.expect("secure session bootstrap should succeed");

    let active_subject: Result<Option<Principal>, Error> = query_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_delegated_session_subject",
        (),
    );
    assert_eq!(
        active_subject.expect("query session subject failed"),
        Some(delegated_subject)
    );

    let verify_after_bootstrap: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_verify_token",
        (token.clone(),),
    );
    verify_after_bootstrap.expect("authenticated guard must honor delegated session subject");

    for method in [
        "signer_guard_is_root",
        "signer_guard_is_controller",
        "signer_guard_is_parent",
        "signer_guard_is_registered_to_subnet",
    ] {
        let denied: Result<(), Error> = update_call_as(&pic, signer_id, wallet, method, ());
        let err = denied.expect_err("raw caller guard must deny wallet caller");
        assert_eq!(err.code, ErrorCode::Unauthorized);
    }

    let cleared: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_clear_delegated_session",
        (),
    );
    cleared.expect("session clear should succeed");

    let denied_after_clear: Result<(), Error> =
        update_call_as(&pic, signer_id, wallet, "signer_verify_token", (token,));
    let err = denied_after_clear.expect_err("subject mismatch must return after clearing session");
    assert_eq!(err.code, ErrorCode::Unauthorized);
    assert!(
        err.message.contains("does not match caller"),
        "expected subject mismatch denial after clear, got: {err:?}"
    );
}

#[test]
fn authenticated_guard_checks_current_proof_before_signature_validation() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);
    let signer_id = signer_pid(&pic, root_id);
    wait_until_ready(&pic, signer_id);

    let wallet = Principal::from_slice(&[92; 29]);
    let now: Result<u64, Error> =
        query_call_as(&pic, root_id, Principal::anonymous(), "root_now_secs", ());
    let now = now.expect("query root now_secs failed");

    let claims_a = DelegatedTokenClaims {
        sub: wallet,
        shard_pid: signer_id,
        scopes: vec![cap::VERIFY.to_string()],
        aud: vec![signer_id],
        iat: now,
        exp: now + 120,
    };
    let token_a: Result<DelegatedToken, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_test_delegated_token",
        (claims_a,),
    );
    let mut token_a = token_a.expect("issue token_a failed");

    let claims_b = DelegatedTokenClaims {
        sub: wallet,
        shard_pid: signer_id,
        scopes: vec![cap::VERIFY.to_string(), "extra".to_string()],
        aud: vec![signer_id],
        iat: now,
        exp: now + 120,
    };
    let token_b: Result<DelegatedToken, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_test_delegated_token",
        (claims_b,),
    );
    let token_b = token_b.expect("issue token_b failed");

    let keys: Result<(Vec<u8>, Vec<u8>), Error> = query_call_as(
        &pic,
        root_id,
        Principal::anonymous(),
        "root_test_delegation_public_keys",
        (),
    );
    let (root_public_key, shard_public_key) = keys.expect("query test delegation keys failed");
    let install_verifier_material: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        root_id,
        "signer_install_test_delegation_material",
        (token_b.proof, root_public_key, shard_public_key),
    );
    install_verifier_material.expect("install signer delegation material must succeed");

    // Make signatures invalid so stage ordering regressions fail this test.
    token_a.proof.cert_sig.clear();
    token_a.token_sig.clear();

    let denied: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_verify_token_any",
        (token_a,),
    );
    let err = denied.expect_err("missing proof must fail before signature checks");
    assert_eq!(err.code, ErrorCode::Unauthorized);
    assert!(
        err.message.contains("delegation proof miss"),
        "expected proof-miss denial, got: {err:?}"
    );
    assert!(
        !err.message.contains("signature unavailable"),
        "expected proof check to run before signature validation, got: {err:?}"
    );
}

#[test]
fn delegation_admin_prewarm_updates_stale_verifier_proof_and_records_metrics() {
    let fixture = delegation_admin_fixture(83);

    install_root_test_delegation_material(
        &fixture.pic,
        fixture.root_id,
        fixture.current_token.proof.clone(),
        fixture.root_public_key.clone(),
        fixture.shard_public_key.clone(),
    );
    install_signer_test_delegation_material(
        &fixture.pic,
        fixture.verifier_id,
        fixture.root_id,
        fixture.stale_token.proof.clone(),
        fixture.root_public_key.clone(),
        fixture.shard_public_key.clone(),
    );

    assert_token_verify_proof_missing(
        &fixture.pic,
        fixture.verifier_id,
        fixture.delegated_subject,
        fixture.current_token.clone(),
    );

    let prewarm = prewarm_verifiers(
        &fixture.pic,
        fixture.root_id,
        fixture.current_token.proof.clone(),
        vec![fixture.verifier_id],
    );
    let DelegationAdminResponse::PrewarmedVerifiers { result } = prewarm else {
        panic!("expected prewarm response");
    };
    assert_eq!(result.results.len(), 1);
    let response = &result.results[0];
    assert_eq!(response.target, fixture.verifier_id);
    assert_eq!(response.status, DelegationProvisionStatus::Ok);
    assert!(
        response.error.is_none(),
        "unexpected prewarm error: {response:?}"
    );

    let verified_after_prewarm: Result<(), Error> = update_call_as(
        &fixture.pic,
        fixture.verifier_id,
        fixture.delegated_subject,
        "signer_verify_token",
        (fixture.current_token,),
    );
    verified_after_prewarm.expect("prewarm should update verifier proof");

    assert_access_metrics(
        &fixture.pic,
        fixture.root_id,
        "auth_signer",
        &[
            ("delegation_install_total{intent=\"prewarm\"}", 1),
            (
                "delegation_install_normalized_target_total{intent=\"prewarm\"}",
                1,
            ),
            (
                "delegation_install_fanout_bucket{intent=\"prewarm\",bucket=\"1\"}",
                1,
            ),
            (
                "delegation_push_attempt{role=\"verifier\",origin=\"prewarm\"}",
                1,
            ),
            (
                "delegation_push_success{role=\"verifier\",origin=\"prewarm\"}",
                1,
            ),
            ("delegation_push_complete{origin=\"prewarm\"}", 1),
        ],
    );
    assert_access_metrics(
        &fixture.pic,
        fixture.verifier_id,
        "auth_verifier",
        &[("token_rejected_proof_miss", 1)],
    );
}

#[test]
fn delegation_admin_repair_requires_matching_local_root_proof() {
    let fixture = delegation_admin_fixture(84);

    install_root_test_delegation_material(
        &fixture.pic,
        fixture.root_id,
        fixture.stale_token.proof,
        fixture.root_public_key,
        fixture.shard_public_key,
    );

    let repair = repair_verifiers(
        &fixture.pic,
        fixture.root_id,
        fixture.current_token.proof,
        vec![fixture.verifier_id],
    );
    let err = repair.expect_err("repair must reject non-local proof redistribution");
    assert_eq!(err.code, ErrorCode::NotFound);
    assert!(
        err.message.contains("existing local proof"),
        "expected repair no-create failure, got: {err:?}"
    );

    assert_access_metrics(
        &fixture.pic,
        fixture.root_id,
        "auth_signer",
        &[
            ("delegation_install_total{intent=\"repair\"}", 1),
            (
                "delegation_install_normalized_target_total{intent=\"repair\"}",
                1,
            ),
            (
                "delegation_install_validation_failed{intent=\"repair\",stage=\"post_normalization\",reason=\"repair_missing_local\"}",
                1,
            ),
            (
                "delegation_push_attempt{role=\"verifier\",origin=\"repair\"}",
                0,
            ),
        ],
    );
}

#[test]
fn verifier_store_rejects_root_push_when_local_canister_is_not_in_proof_audience() {
    let fixture = delegation_admin_fixture(88);

    install_root_test_delegation_material(
        &fixture.pic,
        fixture.root_id,
        fixture.current_token.proof.clone(),
        fixture.root_public_key,
        fixture.shard_public_key,
    );

    let store: Result<(), Error> = update_call_as(
        &fixture.pic,
        fixture.signer_id,
        fixture.root_id,
        "canic_delegation_set_verifier_proof",
        (DelegationProofInstallRequest {
            proof: fixture.current_token.proof,
            intent: DelegationProofInstallIntent::Prewarm,
        },),
    );
    let err = store.expect_err("verifier store must reject proof outside local audience");
    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(
        err.message.contains("not in proof audience"),
        "expected target-side audience rejection, got: {err:?}"
    );

    assert_access_metrics(
        &fixture.pic,
        fixture.signer_id,
        "auth_signer",
        &[(
            "delegation_install_validation_failed{intent=\"prewarm\",stage=\"post_normalization\",reason=\"target_not_in_audience\"}",
            1,
        )],
    );
}

#[test]
fn signer_runtime_prefers_most_recent_keyed_proof_for_signing_selection() {
    let fixture = delegation_admin_fixture(85);

    install_signer_test_delegation_material(
        &fixture.pic,
        fixture.signer_id,
        fixture.root_id,
        fixture.stale_token.proof.clone(),
        fixture.root_public_key.clone(),
        fixture.shard_public_key.clone(),
    );

    let selected_before: Result<Option<canic_core::dto::auth::DelegationProof>, Error> =
        query_call_as(
            &fixture.pic,
            fixture.signer_id,
            Principal::anonymous(),
            "signer_current_signing_proof_test",
            (),
        );
    assert_eq!(
        selected_before.expect("query current signing proof failed"),
        Some(fixture.stale_token.proof.clone()),
        "signer should expose the initially installed proof"
    );

    install_signer_test_delegation_material(
        &fixture.pic,
        fixture.signer_id,
        fixture.root_id,
        fixture.current_token.proof.clone(),
        fixture.root_public_key.clone(),
        fixture.shard_public_key.clone(),
    );

    let selected_after: Result<Option<canic_core::dto::auth::DelegationProof>, Error> =
        query_call_as(
            &fixture.pic,
            fixture.signer_id,
            Principal::anonymous(),
            "signer_current_signing_proof_test",
            (),
        );
    assert_eq!(
        selected_after.expect("query current signing proof failed"),
        Some(fixture.current_token.proof),
        "signer should prefer the newest keyed proof after rotation"
    );
}

#[test]
#[expect(clippy::too_many_lines)]
fn delegation_tier1_issue_verify_bootstrap_authenticated_end_to_end() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);
    let signer_id = signer_pid(&pic, root_id);
    wait_until_ready(&pic, signer_id);

    let issued_attestation: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued_attestation =
        issued_attestation.expect("root role attestation issuance must succeed");
    let issued_at = issued_attestation.payload.issued_at;

    let create_verifier_request = Request::CreateCanister(CreateCanisterRequest {
        canister_role: CanisterRole::new("project_hub"),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
        metadata: None,
    });
    let create_verifier_envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: create_verifier_request.clone(),
        proof: CapabilityProof::RoleAttestation(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: root_capability_hash(root_id, &create_verifier_request),
            attestation: issued_attestation,
        }),
        metadata: capability_metadata(issued_at, 21, 19, 60),
    };
    let create_verifier_response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (create_verifier_envelope,),
    );
    let verifier_id = match create_verifier_response
        .expect("verifier canister creation capability call must succeed")
        .response
    {
        Response::CreateCanister(res) => res.new_canister_pid,
        other => panic!("expected create-canister response, got: {other:?}"),
    };
    wait_until_ready(&pic, verifier_id);

    let wallet = Principal::from_slice(&[61; 29]);
    let delegated_subject = Principal::from_slice(&[62; 29]);
    let now: Result<u64, Error> =
        query_call_as(&pic, root_id, Principal::anonymous(), "root_now_secs", ());
    let now = now.expect("query root now_secs failed");

    let claims = DelegatedTokenClaims {
        sub: delegated_subject,
        shard_pid: signer_id,
        scopes: vec![cap::VERIFY.to_string()],
        aud: vec![verifier_id],
        iat: now,
        exp: now + 120,
    };
    let issued_token: Result<DelegatedToken, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_test_delegated_token",
        (claims,),
    );
    let token = issued_token.expect("test delegation token issuance must succeed");

    let keys: Result<(Vec<u8>, Vec<u8>), Error> = query_call_as(
        &pic,
        root_id,
        Principal::anonymous(),
        "root_test_delegation_public_keys",
        (),
    );
    let (root_public_key, shard_public_key) = keys.expect("query test delegation keys failed");

    let install_signer_material: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        root_id,
        "signer_install_test_delegation_material",
        (
            token.proof.clone(),
            root_public_key.clone(),
            shard_public_key.clone(),
        ),
    );
    install_signer_material.expect("install signer delegation material must succeed");

    let install_verifier_material: Result<(), Error> = update_call_as(
        &pic,
        verifier_id,
        root_id,
        "signer_install_test_delegation_material",
        (token.proof.clone(), root_public_key, shard_public_key),
    );
    install_verifier_material.expect("install verifier delegation material must succeed");

    let verify_on_verifier: Result<(), Error> = update_call_as(
        &pic,
        verifier_id,
        delegated_subject,
        "signer_verify_token",
        (token.clone(),),
    );
    verify_on_verifier.expect("verifier local token check must succeed after provisioning");

    let bootstrap_ok: Result<(), Error> = update_call_as(
        &pic,
        verifier_id,
        wallet,
        "signer_bootstrap_delegated_session",
        (token.clone(), delegated_subject, Some(60u64)),
    );
    bootstrap_ok.expect("delegated session bootstrap must succeed on verifier");

    let active_subject: Result<Option<Principal>, Error> = query_call_as(
        &pic,
        verifier_id,
        wallet,
        "signer_delegated_session_subject",
        (),
    );
    assert_eq!(
        active_subject.expect("query verifier delegated session subject failed"),
        Some(delegated_subject)
    );

    let authenticated_after_bootstrap: Result<(), Error> =
        update_call_as(&pic, verifier_id, wallet, "signer_verify_token", (token,));
    authenticated_after_bootstrap
        .expect("authenticated guard must succeed after verifier bootstrap");
}

#[test]
#[expect(clippy::too_many_lines)]
fn delegated_session_does_not_affect_role_attestation_or_capability_raw_caller_checks() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);
    let signer_id = signer_pid(&pic, root_id);
    wait_until_ready(&pic, signer_id);

    let wallet = Principal::from_slice(&[51; 29]);
    let delegated_subject = Principal::from_slice(&[52; 29]);
    let now: Result<u64, Error> =
        query_call_as(&pic, root_id, Principal::anonymous(), "root_now_secs", ());
    let now = now.expect("query root now_secs failed");

    let claims = DelegatedTokenClaims {
        sub: delegated_subject,
        shard_pid: signer_id,
        scopes: vec![cap::VERIFY.to_string()],
        aud: vec![root_id],
        iat: now,
        exp: now + 120,
    };
    let issued_token: Result<DelegatedToken, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_test_delegated_token",
        (claims,),
    );
    let token = issued_token.expect("token issuance failed");

    let keys: Result<(Vec<u8>, Vec<u8>), Error> = query_call_as(
        &pic,
        root_id,
        Principal::anonymous(),
        "root_test_delegation_public_keys",
        (),
    );
    let (root_public_key, shard_public_key) = keys.expect("query test delegation keys failed");

    let canister_bootstrap_attempt: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        signer_id,
        "root_bootstrap_delegated_session",
        (token.clone(), delegated_subject, Some(60u64)),
    );
    let err = canister_bootstrap_attempt.expect_err("registered canister caller must be rejected");
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert!(
        err.message.contains("wallet caller rejected"),
        "expected wallet-caller rejection, got: {err:?}"
    );

    let stored: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_install_test_delegation_material",
        (token.proof.clone(), root_public_key, shard_public_key),
    );
    stored.expect("installing root verifier proof material should succeed");

    let bootstrap_ok: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        wallet,
        "root_bootstrap_delegated_session",
        (token, delegated_subject, Some(60u64)),
    );
    bootstrap_ok.expect("wallet delegated session bootstrap should succeed");

    let active_subject: Result<Option<Principal>, Error> =
        query_call_as(&pic, root_id, wallet, "root_delegated_session_subject", ());
    assert_eq!(
        active_subject.expect("query root delegated session subject failed"),
        Some(delegated_subject)
    );

    let issued_attestation: Result<SignedRoleAttestation, Error> = update_call_as(
        &pic,
        root_id,
        wallet,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued_attestation = issued_attestation.expect("attestation issuance failed");

    let verify_attestation: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        wallet,
        "root_verify_role_attestation",
        (issued_attestation.clone(), 0u64),
    );
    verify_attestation.expect("role attestation should verify against raw transport caller");

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
            attestation: issued_attestation.clone(),
        }),
        metadata: capability_metadata(issued_attestation.payload.issued_at, 12, 34, 60),
    };

    let capability_response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        wallet,
        "canic_response_capability_v1",
        (envelope,),
    );
    let err =
        capability_response.expect_err("capability should fail for unregistered wallet caller");
    assert!(
        !err.message.contains("subject mismatch"),
        "capability path must not use delegated subject as caller: {err:?}"
    );
    assert!(
        err.message
            .contains("not registered on the subnet registry"),
        "expected raw caller subnet-registry denial, got: {err:?}"
    );
}

#[test]
#[expect(clippy::too_many_lines)]
fn delegated_session_bootstrap_replay_policy_and_metrics() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);
    let signer_id = signer_pid(&pic, root_id);
    wait_until_ready(&pic, signer_id);

    let wallet = Principal::from_slice(&[71; 29]);
    let wallet_other = Principal::from_slice(&[72; 29]);
    let delegated_subject = Principal::from_slice(&[73; 29]);
    let delegated_subject_other = Principal::from_slice(&[74; 29]);

    let now: Result<u64, Error> =
        query_call_as(&pic, root_id, Principal::anonymous(), "root_now_secs", ());
    let now = now.expect("query root now_secs failed");

    let claims_a = DelegatedTokenClaims {
        sub: delegated_subject,
        shard_pid: signer_id,
        scopes: vec![cap::VERIFY.to_string()],
        aud: vec![signer_id],
        iat: now,
        exp: now + 120,
    };
    let token_a: Result<DelegatedToken, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_test_delegated_token",
        (claims_a,),
    );
    let token_a = token_a.expect("token_a issuance failed");

    let keys: Result<(Vec<u8>, Vec<u8>), Error> = query_call_as(
        &pic,
        root_id,
        Principal::anonymous(),
        "root_test_delegation_public_keys",
        (),
    );
    let (root_public_key, shard_public_key) = keys.expect("query test delegation keys failed");
    let install_signer_material_a: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        root_id,
        "signer_install_test_delegation_material",
        (
            token_a.proof.clone(),
            root_public_key.clone(),
            shard_public_key.clone(),
        ),
    );
    install_signer_material_a.expect("install signer proof A should succeed");

    let bootstrap_a: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_bootstrap_delegated_session",
        (token_a.clone(), delegated_subject, Some(60u64)),
    );
    bootstrap_a.expect("initial bootstrap should succeed");
    assert_eq!(
        access_metric_count(&pic, signer_id, "auth_session", "session_created"),
        1
    );
    assert_eq!(
        access_metric_count(&pic, signer_id, "auth_session", "session_replaced"),
        0
    );

    let bootstrap_a_repeat: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_bootstrap_delegated_session",
        (token_a.clone(), delegated_subject, Some(60u64)),
    );
    bootstrap_a_repeat
        .expect("same-token replay with active matching session should be idempotent");
    assert_eq!(
        access_metric_count(
            &pic,
            signer_id,
            "auth_session",
            "session_bootstrap_replay_idempotent"
        ),
        1
    );
    assert_eq!(
        access_metric_count(&pic, signer_id, "auth_session", "session_created"),
        1
    );
    assert_eq!(
        access_metric_count(&pic, signer_id, "auth_session", "session_replaced"),
        0
    );

    let mismatch: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_bootstrap_delegated_session",
        (token_a.clone(), delegated_subject_other, Some(60u64)),
    );
    let mismatch_err =
        mismatch.expect_err("same wallet with different delegated subject must fail closed");
    assert_eq!(mismatch_err.code, ErrorCode::Forbidden);
    assert!(
        mismatch_err.message.contains("subject mismatch"),
        "expected subject mismatch rejection, got: {mismatch_err:?}"
    );
    assert_eq!(
        access_metric_count(
            &pic,
            signer_id,
            "auth_session",
            "session_bootstrap_rejected_subject_mismatch"
        ),
        1
    );

    let clear: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_clear_delegated_session",
        (),
    );
    clear.expect("clear delegated session should succeed");

    let replay_after_clear: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_bootstrap_delegated_session",
        (token_a.clone(), delegated_subject, Some(60u64)),
    );
    let replay_after_clear_err =
        replay_after_clear.expect_err("same token replay after clear should be rejected");
    assert_eq!(replay_after_clear_err.code, ErrorCode::Forbidden);
    assert!(
        replay_after_clear_err.message.contains("replay rejected"),
        "expected replay rejection after clear, got: {replay_after_clear_err:?}"
    );
    assert_eq!(
        access_metric_count(
            &pic,
            signer_id,
            "auth_session",
            "session_bootstrap_rejected_replay_reused"
        ),
        1
    );

    let replay_other_wallet: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet_other,
        "signer_bootstrap_delegated_session",
        (token_a, delegated_subject, Some(60u64)),
    );
    let replay_other_wallet_err =
        replay_other_wallet.expect_err("same token replay from another wallet should be rejected");
    assert_eq!(replay_other_wallet_err.code, ErrorCode::Forbidden);
    assert!(
        replay_other_wallet_err.message.contains("already bound"),
        "expected replay-conflict rejection, got: {replay_other_wallet_err:?}"
    );
    assert_eq!(
        access_metric_count(
            &pic,
            signer_id,
            "auth_session",
            "session_bootstrap_rejected_replay_conflict"
        ),
        1
    );

    let claims_b = DelegatedTokenClaims {
        sub: delegated_subject,
        shard_pid: signer_id,
        scopes: vec![cap::VERIFY.to_string()],
        aud: vec![signer_id],
        iat: now,
        exp: now + 180,
    };
    let token_b: Result<DelegatedToken, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_test_delegated_token",
        (claims_b,),
    );
    let token_b = token_b.expect("token_b issuance failed");

    let install_signer_material_b: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        root_id,
        "signer_install_test_delegation_material",
        (
            token_b.proof.clone(),
            root_public_key.clone(),
            shard_public_key.clone(),
        ),
    );
    install_signer_material_b.expect("install signer proof B should succeed");

    let bootstrap_b: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_bootstrap_delegated_session",
        (token_b, delegated_subject, Some(60u64)),
    );
    bootstrap_b.expect("fresh token should create session state after clear");
    assert_eq!(
        access_metric_count(&pic, signer_id, "auth_session", "session_created"),
        2
    );

    let claims_c = DelegatedTokenClaims {
        sub: delegated_subject,
        shard_pid: signer_id,
        scopes: vec![cap::VERIFY.to_string()],
        aud: vec![signer_id],
        iat: now,
        exp: now + 240,
    };
    let token_c: Result<DelegatedToken, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_test_delegated_token",
        (claims_c,),
    );
    let token_c = token_c.expect("token_c issuance failed");

    let install_signer_material_c: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        root_id,
        "signer_install_test_delegation_material",
        (token_c.proof.clone(), root_public_key, shard_public_key),
    );
    install_signer_material_c.expect("install signer proof C should succeed");

    let bootstrap_c: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_bootstrap_delegated_session",
        (token_c, delegated_subject, Some(60u64)),
    );
    bootstrap_c.expect("fresh token with active session should replace session state");
    assert_eq!(
        access_metric_count(&pic, signer_id, "auth_session", "session_replaced"),
        1
    );
}

#[test]
fn delegated_session_bootstrap_replay_with_expired_token_fails_closed() {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);
    let signer_id = signer_pid(&pic, root_id);
    wait_until_ready(&pic, signer_id);

    let wallet = Principal::from_slice(&[81; 29]);
    let delegated_subject = Principal::from_slice(&[82; 29]);

    let now: Result<u64, Error> =
        query_call_as(&pic, root_id, Principal::anonymous(), "root_now_secs", ());
    let now = now.expect("query root now_secs failed");

    let claims = DelegatedTokenClaims {
        sub: delegated_subject,
        shard_pid: signer_id,
        scopes: vec![cap::VERIFY.to_string()],
        aud: vec![signer_id],
        iat: now,
        exp: now + 5,
    };
    let token: Result<DelegatedToken, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_test_delegated_token",
        (claims,),
    );
    let token = token.expect("token issuance failed");

    let keys: Result<(Vec<u8>, Vec<u8>), Error> = query_call_as(
        &pic,
        root_id,
        Principal::anonymous(),
        "root_test_delegation_public_keys",
        (),
    );
    let (root_public_key, shard_public_key) = keys.expect("query test delegation keys failed");
    let install_signer_material: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        root_id,
        "signer_install_test_delegation_material",
        (token.proof.clone(), root_public_key, shard_public_key),
    );
    install_signer_material.expect("install signer proof should succeed");

    let bootstrap_ok: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_bootstrap_delegated_session",
        (token.clone(), delegated_subject, Some(5u64)),
    );
    bootstrap_ok.expect("initial bootstrap should succeed before token expiry");

    pic.advance_time(Duration::from_secs(6));
    pic.tick();

    let expired_replay: Result<(), Error> = update_call_as(
        &pic,
        signer_id,
        wallet,
        "signer_bootstrap_delegated_session",
        (token, delegated_subject, Some(5u64)),
    );
    expired_replay.expect_err("expired replay must fail closed");
    assert_eq!(
        access_metric_count(
            &pic,
            signer_id,
            "auth_session",
            "session_bootstrap_rejected_token_invalid"
        ),
        1
    );
}

#[test]
fn test_delegation_material_install_hook_not_compiled_in_normal_build() {
    let workspace_root = workspace_root();
    build_canisters_without_test_material_once(&workspace_root);
    let root_wasm = read_wasm_from_target(
        &test_target_dir_without_test_material(&workspace_root),
        "delegation_root_stub",
    );

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);
    let signer_id = signer_pid(&pic, root_id);
    wait_until_ready(&pic, signer_id);

    let now: Result<u64, Error> =
        query_call_as(&pic, root_id, Principal::anonymous(), "root_now_secs", ());
    let now = now.expect("query root now_secs failed");

    let claims = DelegatedTokenClaims {
        sub: Principal::from_slice(&[61; 29]),
        shard_pid: signer_id,
        scopes: vec![cap::VERIFY.to_string()],
        aud: vec![root_id],
        iat: now,
        exp: now + 120,
    };
    let issued_token: Result<DelegatedToken, Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_issue_test_delegated_token",
        (claims,),
    );
    let token = issued_token.expect("token issuance failed");

    let keys: Result<(Vec<u8>, Vec<u8>), Error> = query_call_as(
        &pic,
        root_id,
        Principal::anonymous(),
        "root_test_delegation_public_keys",
        (),
    );
    let (root_public_key, shard_public_key) = keys.expect("query test delegation keys failed");

    let install = update_call_raw_as(
        &pic,
        root_id,
        root_id,
        "root_install_test_delegation_material",
        (token.proof, root_public_key, shard_public_key),
    );
    let err = install.expect_err("normal build must not compile test delegation-material install");
    let normalized = err.to_ascii_lowercase();
    assert!(
        normalized.contains("method") && normalized.contains("not")
            || normalized.contains("not found")
            || normalized.contains("has no update method"),
        "expected missing-method failure, got: {err}"
    );
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

///
/// DelegationAdminFixture
///

struct DelegationAdminFixture {
    pic: SerialPic,
    root_id: Principal,
    signer_id: Principal,
    verifier_id: Principal,
    delegated_subject: Principal,
    stale_token: DelegatedToken,
    current_token: DelegatedToken,
    root_public_key: Vec<u8>,
    shard_public_key: Vec<u8>,
}

// Build a reusable root/signer/verifier setup with two proof generations.
fn delegation_admin_fixture(subject_seed: u8) -> DelegationAdminFixture {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");

    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);
    let signer_id = signer_pid(&pic, root_id);
    let verifier_id = create_verifier_canister(&pic, root_id);
    wait_until_ready(&pic, signer_id);

    let delegated_subject = Principal::from_slice(&[subject_seed; 29]);
    let stale_token =
        issue_test_delegated_token(&pic, root_id, signer_id, verifier_id, delegated_subject, 60);
    let current_token = issue_test_delegated_token(
        &pic,
        root_id,
        signer_id,
        verifier_id,
        delegated_subject,
        120,
    );
    let (root_public_key, shard_public_key) = delegation_public_keys(&pic, root_id);

    DelegationAdminFixture {
        pic,
        root_id,
        signer_id,
        verifier_id,
        delegated_subject,
        stale_token,
        current_token,
        root_public_key,
        shard_public_key,
    }
}

// Issue a test delegated token for the requested verifier audience and TTL.
fn issue_test_delegated_token(
    pic: &pocket_ic::PocketIc,
    root_id: Principal,
    signer_id: Principal,
    verifier_id: Principal,
    delegated_subject: Principal,
    ttl_seconds: u64,
) -> DelegatedToken {
    let now: Result<u64, Error> =
        query_call_as(pic, root_id, Principal::anonymous(), "root_now_secs", ());
    let now = now.expect("query root now_secs failed");
    let claims = DelegatedTokenClaims {
        sub: delegated_subject,
        shard_pid: signer_id,
        scopes: vec![cap::VERIFY.to_string()],
        aud: vec![verifier_id],
        iat: now,
        exp: now + ttl_seconds,
    };
    let issued_token: Result<DelegatedToken, Error> = update_call_as(
        pic,
        root_id,
        root_id,
        "root_issue_test_delegated_token",
        (claims,),
    );

    issued_token.expect("delegated token issuance failed")
}

// Query the root test public keys used for proof installation hooks.
fn delegation_public_keys(pic: &pocket_ic::PocketIc, root_id: Principal) -> (Vec<u8>, Vec<u8>) {
    let keys: Result<(Vec<u8>, Vec<u8>), Error> = query_call_as(
        pic,
        root_id,
        Principal::anonymous(),
        "root_test_delegation_public_keys",
        (),
    );

    keys.expect("query test delegation keys failed")
}

// Install proof material into the root verifier test hook.
fn install_root_test_delegation_material(
    pic: &pocket_ic::PocketIc,
    root_id: Principal,
    proof: canic_core::dto::auth::DelegationProof,
    root_public_key: Vec<u8>,
    shard_public_key: Vec<u8>,
) {
    let install: Result<(), Error> = update_call_as(
        pic,
        root_id,
        root_id,
        "root_install_test_delegation_material",
        (proof, root_public_key, shard_public_key),
    );

    install.expect("root test delegation material install must succeed");
}

// Install proof material into a signer/verifier test hook.
fn install_signer_test_delegation_material(
    pic: &pocket_ic::PocketIc,
    canister_id: Principal,
    caller: Principal,
    proof: canic_core::dto::auth::DelegationProof,
    root_public_key: Vec<u8>,
    shard_public_key: Vec<u8>,
) {
    let install: Result<(), Error> = update_call_as(
        pic,
        canister_id,
        caller,
        "signer_install_test_delegation_material",
        (proof, root_public_key, shard_public_key),
    );

    install.expect("signer delegation material install must succeed");
}

// Verify that keyed lookup fails as a proof miss before any prewarm repair.
fn assert_token_verify_proof_missing(
    pic: &pocket_ic::PocketIc,
    verifier_id: Principal,
    delegated_subject: Principal,
    token: DelegatedToken,
) {
    let denied: Result<(), Error> = update_call_as(
        pic,
        verifier_id,
        delegated_subject,
        "signer_verify_token",
        (token,),
    );
    let err = denied.expect_err("stale verifier proof must fail closed");
    assert_eq!(err.code, ErrorCode::Unauthorized);
    assert!(
        err.message.contains("delegation proof miss"),
        "expected proof-miss denial, got: {err:?}"
    );
}

// Dispatch a root prewarm admin command and decode the typed response.
fn prewarm_verifiers(
    pic: &pocket_ic::PocketIc,
    root_id: Principal,
    proof: canic_core::dto::auth::DelegationProof,
    verifier_targets: Vec<Principal>,
) -> DelegationAdminResponse {
    let prewarm: Result<DelegationAdminResponse, Error> = update_call_as(
        pic,
        root_id,
        Principal::anonymous(),
        "canic_delegation_admin",
        (DelegationAdminCommand::PrewarmVerifiers(
            DelegationVerifierProofPushRequest {
                proof,
                verifier_targets,
            },
        ),),
    );

    prewarm.expect("prewarm admin call must succeed")
}

// Dispatch a root repair admin command and preserve the typed error surface.
fn repair_verifiers(
    pic: &pocket_ic::PocketIc,
    root_id: Principal,
    proof: canic_core::dto::auth::DelegationProof,
    verifier_targets: Vec<Principal>,
) -> Result<DelegationAdminResponse, Error> {
    update_call_as(
        pic,
        root_id,
        Principal::anonymous(),
        "canic_delegation_admin",
        (DelegationAdminCommand::RepairVerifiers(
            DelegationVerifierProofPushRequest {
                proof,
                verifier_targets,
            },
        ),),
    )
}

// Assert a batch of access-metric predicates for a single canister endpoint.
fn assert_access_metrics(
    pic: &pocket_ic::PocketIc,
    canister_id: Principal,
    endpoint: &str,
    expected: &[(&str, u64)],
) {
    for (predicate, count) in expected {
        assert_eq!(
            access_metric_count(pic, canister_id, endpoint, predicate),
            *count,
            "unexpected metric count for {endpoint} / {predicate}"
        );
    }
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

fn update_call_raw_as<A>(
    pic: &pocket_ic::PocketIc,
    canister_id: Principal,
    caller: Principal,
    method: &str,
    args: A,
) -> Result<Vec<u8>, String>
where
    A: ArgumentEncoder,
{
    let payload = encode_args(args).expect("encode args");
    pic.update_call(canister_id, caller, method, payload)
        .map_err(|err| err.to_string())
}

fn query_call_as<T, A>(
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
        .query_call(canister_id, caller, method, payload)
        .expect("query_call failed");

    decode_one(&result).expect("decode response")
}

fn access_metric_count(
    pic: &pocket_ic::PocketIc,
    canister_id: Principal,
    endpoint: &str,
    predicate: &str,
) -> u64 {
    let response: Result<MetricsResponse, Error> = query_call_as(
        pic,
        canister_id,
        Principal::anonymous(),
        "canic_metrics",
        (MetricsRequest {
            kind: MetricsKind::Access,
            page: PageRequest {
                limit: 10_000,
                offset: 0,
            },
        },),
    );
    let response = response.expect("query canic_metrics failed");
    let MetricsResponse::Access(page) = response else {
        panic!("expected access metrics response");
    };

    page.entries
        .into_iter()
        .find_map(|entry| {
            if entry.endpoint == endpoint && entry.predicate == predicate {
                Some(entry.count)
            } else {
                None
            }
        })
        .unwrap_or(0)
}

// Create a non-root verifier canister through the root capability endpoint.
fn create_verifier_canister(pic: &pocket_ic::PocketIc, root_id: Principal) -> Principal {
    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");
    let issued_at = issued.payload.issued_at;

    let request = Request::CreateCanister(CreateCanisterRequest {
        canister_role: CanisterRole::new("project_hub"),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
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
        metadata: capability_metadata(issued_at, 41, 24, 60),
    };
    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let verifier_id = match response
        .expect("verifier canister creation capability call must succeed")
        .response
    {
        Response::CreateCanister(res) => res.new_canister_pid,
        other => panic!("expected create-canister response, got: {other:?}"),
    };
    wait_until_ready(pic, verifier_id);
    verifier_id
}

fn signer_pid(pic: &pocket_ic::PocketIc, root_id: Principal) -> Principal {
    for _ in 0..120 {
        let registry: Result<SubnetRegistryResponse, Error> = query_call_as(
            pic,
            root_id,
            Principal::anonymous(),
            "canic_subnet_registry",
            (),
        );

        if let Ok(registry) = registry
            && let Some(pid) = registry
                .0
                .into_iter()
                .find(|entry| entry.role == CanisterRole::new("signer"))
                .map(|entry| entry.pid)
        {
            return pid;
        }

        pic.tick();
    }

    panic!("signer canister must be registered");
}

fn wait_until_ready(pic: &pocket_ic::PocketIc, canister_id: Principal) {
    let payload = encode_args(()).expect("encode empty args");
    for _ in 0..240 {
        if let Ok(bytes) = pic.query_call(
            canister_id,
            Principal::anonymous(),
            "canic_ready",
            payload.clone(),
        ) && let Ok(ready) = decode_one::<bool>(&bytes)
            && ready
        {
            return;
        }
        pic.tick();
    }

    panic!("canister did not report ready in time: {canister_id}");
}

fn bogus_delegated_token(root_pid: Principal, shard_pid: Principal) -> DelegatedToken {
    let user = Principal::from_slice(&[77; 29]);
    DelegatedToken {
        claims: DelegatedTokenClaims {
            sub: user,
            shard_pid,
            aud: vec![root_pid],
            scopes: vec![cap::VERIFY.to_string()],
            iat: 1,
            exp: 2,
        },
        proof: canic_core::dto::auth::DelegationProof {
            cert: canic_core::dto::auth::DelegationCert {
                root_pid,
                shard_pid,
                issued_at: 1,
                expires_at: 2,
                scopes: vec![cap::VERIFY.to_string()],
                aud: vec![root_pid],
            },
            cert_sig: vec![0],
        },
        token_sig: vec![0],
    }
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

// Build the test canisters with delegation-material test cfg enabled.
// This path is used by the main delegated-session regression suite.
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
        // Activate compile-time test delegation-material hooks for PIC canisters.
        cmd.env("CANIC_TEST_DELEGATION_MATERIAL", "1");
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

// Build the same test canisters without delegation-material test cfg enabled.
// This validates that normal builds do not compile the install hook.
fn build_canisters_without_test_material_once(workspace_root: &PathBuf) {
    BUILD_WITHOUT_TEST_MATERIAL_ONCE.call_once_force(|_| {
        let target_dir = test_target_dir_without_test_material(workspace_root);
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

fn read_wasm_from_target(target_dir: &Path, crate_name: &str) -> Vec<u8> {
    let wasm_path = wasm_path_from_target(target_dir, crate_name);
    fs::read(&wasm_path).unwrap_or_else(|err| panic!("failed to read {crate_name} wasm: {err}"))
}

fn wasm_path(workspace_root: &Path, crate_name: &str) -> PathBuf {
    if let Some(dir) = prebuilt_wasm_dir() {
        return dir.join(format!("{crate_name}.wasm"));
    }

    let target_dir = test_target_dir(workspace_root);

    wasm_path_from_target(&target_dir, crate_name)
}

fn wasm_path_from_target(target_dir: &Path, crate_name: &str) -> PathBuf {
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

fn test_target_dir_without_test_material(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join("target")
        .join("pic-wasm-no-test-material")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}
