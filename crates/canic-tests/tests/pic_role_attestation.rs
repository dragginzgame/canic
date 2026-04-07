// Category C - Artifact / deployment test (embedded static config).
// This test relies on embedded config by design (test stub).

use candid::{Principal, utils::ArgumentEncoder};
use canic_core::api::rpc::RpcApi;
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
    metrics::{MetricEntry, MetricValue, MetricsKind},
    page::{Page, PageRequest},
    rpc::{CyclesRequest, Request, Response},
};
use canic_core::ids::{CanisterRole, cap};
use canic_testing_internal::pic::{
    CachedInstalledRoot, install_test_root_cached, install_test_root_with_verifier_cached,
    install_test_root_without_test_material_cached, signer_pid,
};
use canic_testkit::pic::{Pic, wait_until_ready as wait_for_ready_canister};
use serde::de::DeserializeOwned;
use std::{
    io::Write,
    ops::Deref,
    sync::{Mutex, OnceLock},
    time::Duration,
};

static DELEGATION_ADMIN_FIXTURE_CACHE: OnceLock<Mutex<Option<DelegationAdminCachedData>>> =
    OnceLock::new();

struct PicBorrow<'a>(&'a Pic);

impl Deref for PicBorrow<'_> {
    type Target = Pic;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

///
/// DelegationAdminCachedData
///

#[derive(Clone)]
struct DelegationAdminCachedData {
    root_id: Principal,
    signer_id: Principal,
    verifier_id: Principal,
    delegated_subject: Principal,
    stale_token: DelegatedToken,
    current_token: DelegatedToken,
    root_public_key: Vec<u8>,
    shard_public_key: Vec<u8>,
}

// Emit one short progress marker for long grouped PocketIC scenario tests.
fn test_progress(test_name: &str, phase: &str) {
    eprintln!("[pic_role_attestation] {test_name}: {phase}");
    let _ = std::io::stderr().flush();
}

fn encode_role_attestation_capability_proof(proof: RoleAttestationProof) -> CapabilityProof {
    proof
        .try_into()
        .expect("role attestation proof should encode")
}

fn encode_delegated_grant_capability_proof(proof: DelegatedGrantProof) -> CapabilityProof {
    proof
        .try_into()
        .expect("delegated grant proof should encode")
}

#[test]
fn role_attestation_verification_paths() {
    let setup = install_test_root_cached();
    let pic = PicBorrow(setup.pic.pic());
    let root_id = setup.root_id;

    // Happy path should verify a freshly issued self-attestation.
    let issued = issue_self_attestation(&pic, root_id, 60, Some(root_id));
    let verified: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (issued, 0u64),
    );
    verified.expect("attestation verification failed");

    // Mismatched caller must fail even with an otherwise valid attestation.
    let issued = issue_self_attestation(&pic, root_id, 60, Some(root_id));
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

    // Audience binding must be enforced by the verifier.
    let wrong_audience = Principal::from_slice(&[9; 29]);
    let issued = issue_self_attestation(&pic, root_id, 60, Some(wrong_audience));
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

    // Epoch floors higher than the attestation epoch must fail closed.
    let issued = issue_self_attestation(&pic, root_id, 60, Some(root_id));
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

    // Expiry is time-sensitive, so keep it last after advancing the clock.
    let issued = issue_self_attestation(&pic, root_id, 1, Some(root_id));
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
fn role_attestation_verify_handles_rotated_key_grace_window() {
    let setup = install_test_root_cached();
    let pic = PicBorrow(setup.pic.pic());
    let root_id = setup.root_id;

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
    test_progress(
        "delegated_session_bootstrap_affects_authenticated_guard_only",
        "setup root",
    );
    let setup = install_test_root_cached();
    let pic = PicBorrow(setup.pic.pic());
    let root_id = setup.root_id;
    let signer_id = setup.signer_id;

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

    test_progress(
        "delegated_session_bootstrap_affects_authenticated_guard_only",
        "verify guard behavior before and after bootstrap",
    );
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
    test_progress(
        "delegated_session_bootstrap_affects_authenticated_guard_only",
        "done",
    );
}

#[test]
fn authenticated_guard_checks_current_proof_before_signature_validation() {
    test_progress(
        "authenticated_guard_checks_current_proof_before_signature_validation",
        "setup root",
    );
    let setup = install_test_root_cached();
    let pic = PicBorrow(setup.pic.pic());
    let root_id = setup.root_id;
    let signer_id = setup.signer_id;

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

    test_progress(
        "authenticated_guard_checks_current_proof_before_signature_validation",
        "proof miss before signature validation",
    );
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
    test_progress(
        "authenticated_guard_checks_current_proof_before_signature_validation",
        "done",
    );
}

#[test]
fn delegation_admin_prewarm_updates_stale_verifier_proof_and_records_metrics() {
    test_progress(
        "delegation_admin_prewarm_updates_stale_verifier_proof_and_records_metrics",
        "setup fixture",
    );
    let fixture = delegation_admin_fixture(83);

    test_progress(
        "delegation_admin_prewarm_updates_stale_verifier_proof_and_records_metrics",
        "install root and stale verifier proof",
    );
    install_root_test_delegation_material(
        fixture.setup.pic.pic(),
        fixture.root_id,
        fixture.current_token.proof.clone(),
        fixture.root_public_key.clone(),
        fixture.shard_public_key.clone(),
    );
    install_signer_test_delegation_material(
        fixture.setup.pic.pic(),
        fixture.verifier_id,
        fixture.root_id,
        fixture.stale_token.proof.clone(),
        fixture.root_public_key.clone(),
        fixture.shard_public_key.clone(),
    );

    assert_token_verify_proof_missing(
        fixture.setup.pic.pic(),
        fixture.verifier_id,
        fixture.delegated_subject,
        fixture.current_token.clone(),
    );

    test_progress(
        "delegation_admin_prewarm_updates_stale_verifier_proof_and_records_metrics",
        "prewarm verifier",
    );
    let prewarm = prewarm_verifiers(
        fixture.setup.pic.pic(),
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
        fixture.setup.pic.pic(),
        fixture.verifier_id,
        fixture.delegated_subject,
        "signer_verify_token",
        (fixture.current_token,),
    );
    verified_after_prewarm.expect("prewarm should update verifier proof");

    assert_access_metrics(
        fixture.setup.pic.pic(),
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
        fixture.setup.pic.pic(),
        fixture.verifier_id,
        "auth_verifier",
        &[("token_rejected_proof_miss", 1)],
    );
    test_progress(
        "delegation_admin_prewarm_updates_stale_verifier_proof_and_records_metrics",
        "done",
    );
}

#[test]
fn delegation_admin_repair_requires_matching_local_root_proof() {
    test_progress(
        "delegation_admin_repair_requires_matching_local_root_proof",
        "setup fixture",
    );
    let fixture = delegation_admin_fixture(84);

    install_root_test_delegation_material(
        fixture.setup.pic.pic(),
        fixture.root_id,
        fixture.stale_token.proof,
        fixture.root_public_key,
        fixture.shard_public_key,
    );

    test_progress(
        "delegation_admin_repair_requires_matching_local_root_proof",
        "repair verifier with mismatched local proof",
    );
    let repair = repair_verifiers(
        fixture.setup.pic.pic(),
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
        fixture.setup.pic.pic(),
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
    test_progress(
        "delegation_admin_repair_requires_matching_local_root_proof",
        "done",
    );
}

#[test]
fn verifier_store_rejects_root_push_when_local_canister_is_not_in_proof_audience() {
    test_progress(
        "verifier_store_rejects_root_push_when_local_canister_is_not_in_proof_audience",
        "setup fixture",
    );
    let fixture = delegation_admin_fixture(88);

    install_root_test_delegation_material(
        fixture.setup.pic.pic(),
        fixture.root_id,
        fixture.current_token.proof.clone(),
        fixture.root_public_key,
        fixture.shard_public_key,
    );

    test_progress(
        "verifier_store_rejects_root_push_when_local_canister_is_not_in_proof_audience",
        "push verifier proof outside audience",
    );
    let store: Result<(), Error> = update_call_as(
        fixture.setup.pic.pic(),
        fixture.signer_id,
        fixture.root_id,
        "canic_delegation_set_verifier_proof",
        (DelegationProofInstallRequest {
            proof: fixture.current_token.proof,
            intent: DelegationProofInstallIntent::Prewarm,
            root_public_key_sec1: None,
            shard_public_key_sec1: None,
        },),
    );
    let err = store.expect_err("verifier store must reject proof outside local audience");
    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(
        err.message.contains("not in proof audience"),
        "expected target-side audience rejection, got: {err:?}"
    );

    assert_access_metrics(
        fixture.setup.pic.pic(),
        fixture.signer_id,
        "auth_signer",
        &[(
            "delegation_install_validation_failed{intent=\"prewarm\",stage=\"post_normalization\",reason=\"target_not_in_audience\"}",
            1,
        )],
    );
    test_progress(
        "verifier_store_rejects_root_push_when_local_canister_is_not_in_proof_audience",
        "done",
    );
}

#[test]
fn signer_runtime_prefers_most_recent_keyed_proof_for_signing_selection() {
    test_progress(
        "signer_runtime_prefers_most_recent_keyed_proof_for_signing_selection",
        "setup fixture",
    );
    let fixture = delegation_admin_fixture(85);

    test_progress(
        "signer_runtime_prefers_most_recent_keyed_proof_for_signing_selection",
        "install stale signing proof",
    );
    install_signer_test_delegation_material(
        fixture.setup.pic.pic(),
        fixture.signer_id,
        fixture.root_id,
        fixture.stale_token.proof.clone(),
        fixture.root_public_key.clone(),
        fixture.shard_public_key.clone(),
    );

    let selected_before: Result<Option<canic_core::dto::auth::DelegationProof>, Error> =
        query_call_as(
            fixture.setup.pic.pic(),
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

    test_progress(
        "signer_runtime_prefers_most_recent_keyed_proof_for_signing_selection",
        "install current signing proof",
    );
    install_signer_test_delegation_material(
        fixture.setup.pic.pic(),
        fixture.signer_id,
        fixture.root_id,
        fixture.current_token.proof.clone(),
        fixture.root_public_key.clone(),
        fixture.shard_public_key.clone(),
    );

    let selected_after: Result<Option<canic_core::dto::auth::DelegationProof>, Error> =
        query_call_as(
            fixture.setup.pic.pic(),
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
    test_progress(
        "signer_runtime_prefers_most_recent_keyed_proof_for_signing_selection",
        "done",
    );
}

#[test]
#[expect(clippy::too_many_lines)]
fn delegation_tier1_issue_verify_bootstrap_authenticated_end_to_end() {
    test_progress(
        "delegation_tier1_issue_verify_bootstrap_authenticated_end_to_end",
        "setup cached verifier baseline",
    );
    let setup = install_test_root_with_verifier_cached();
    let pic = PicBorrow(setup.pic.pic());
    let root_id = setup.root_id;
    let signer_id = signer_pid(&pic, root_id);
    wait_for_ready_canister(&pic, signer_id, 240);
    let verifier_id = setup
        .verifier_id
        .expect("cached verifier baseline must include verifier");
    wait_for_ready_canister(&pic, verifier_id, 240);

    test_progress(
        "delegation_tier1_issue_verify_bootstrap_authenticated_end_to_end",
        "issue delegated token and install proof material",
    );
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

    test_progress(
        "delegation_tier1_issue_verify_bootstrap_authenticated_end_to_end",
        "verify token and bootstrap session",
    );
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
    test_progress(
        "delegation_tier1_issue_verify_bootstrap_authenticated_end_to_end",
        "done",
    );
}

#[test]
#[expect(clippy::too_many_lines)]
fn delegated_session_does_not_affect_role_attestation_or_capability_raw_caller_checks() {
    test_progress(
        "delegated_session_does_not_affect_role_attestation_or_capability_raw_caller_checks",
        "setup root",
    );
    let setup = install_test_root_cached();
    let pic = PicBorrow(setup.pic.pic());
    let root_id = setup.root_id;
    let signer_id = setup.signer_id;

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

    test_progress(
        "delegated_session_does_not_affect_role_attestation_or_capability_raw_caller_checks",
        "reject canister bootstrap caller",
    );
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

    test_progress(
        "delegated_session_does_not_affect_role_attestation_or_capability_raw_caller_checks",
        "bootstrap wallet session and verify raw caller semantics",
    );
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
        proof: encode_role_attestation_capability_proof(RoleAttestationProof {
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
    test_progress(
        "delegated_session_does_not_affect_role_attestation_or_capability_raw_caller_checks",
        "done",
    );
}

#[test]
#[expect(clippy::too_many_lines)]
fn delegated_session_bootstrap_replay_policy_and_metrics() {
    test_progress(
        "delegated_session_bootstrap_replay_policy_and_metrics",
        "setup root",
    );
    let setup = install_test_root_cached();
    let pic = PicBorrow(setup.pic.pic());
    let root_id = setup.root_id;
    let signer_id = setup.signer_id;

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

    test_progress(
        "delegated_session_bootstrap_replay_policy_and_metrics",
        "bootstrap and replay token A",
    );
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

    test_progress(
        "delegated_session_bootstrap_replay_policy_and_metrics",
        "clear session and reject replay reuse",
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

    test_progress(
        "delegated_session_bootstrap_replay_policy_and_metrics",
        "issue fresh tokens B and C",
    );
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
    test_progress(
        "delegated_session_bootstrap_replay_policy_and_metrics",
        "done",
    );
}

#[test]
fn delegated_session_bootstrap_replay_with_expired_token_fails_closed() {
    test_progress(
        "delegated_session_bootstrap_replay_with_expired_token_fails_closed",
        "setup root",
    );
    let setup = install_test_root_cached();
    let pic = PicBorrow(setup.pic.pic());
    let root_id = setup.root_id;
    let signer_id = setup.signer_id;

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

    test_progress(
        "delegated_session_bootstrap_replay_with_expired_token_fails_closed",
        "bootstrap then expire token",
    );
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
    test_progress(
        "delegated_session_bootstrap_replay_with_expired_token_fails_closed",
        "done",
    );
}

#[test]
fn test_delegation_material_install_hook_not_compiled_in_normal_build() {
    test_progress(
        "test_delegation_material_install_hook_not_compiled_in_normal_build",
        "setup cached normal-build root",
    );
    let setup = install_test_root_without_test_material_cached();
    let pic = PicBorrow(setup.pic.pic());
    let root_id = setup.root_id;
    let signer_id = signer_pid(&pic, root_id);
    wait_for_ready_canister(&pic, signer_id, 240);

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
    test_progress(
        "test_delegation_material_install_hook_not_compiled_in_normal_build",
        "done",
    );
}

#[test]
#[expect(clippy::too_many_lines)]
fn capability_endpoint_role_attestation_proof_paths() {
    test_progress(
        "capability_endpoint_role_attestation_proof_paths",
        "setup root",
    );
    let setup = install_test_root_cached();
    let pic = PicBorrow(setup.pic.pic());
    let root_id = setup.root_id;
    let signer_id = setup.signer_id;
    let request = Request::Cycles(CyclesRequest {
        cycles: 1,
        metadata: None,
    });

    test_progress(
        "capability_endpoint_role_attestation_proof_paths",
        "valid cycles proof",
    );
    // A valid child caller with a root-audience attestation should authorize the cycles request.
    let issued = issue_self_attestation_as(&pic, root_id, signer_id, 60, Some(root_id));
    let issued_at = issued.payload.issued_at;
    let envelope =
        cycles_role_attestation_envelope(root_id, request.clone(), issued, issued_at, 1, 9);
    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        signer_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let response = response.expect("capability endpoint call failed");
    match response.response {
        Response::Cycles(res) => assert_eq!(res.cycles_transferred, 1),
        other => panic!("expected cycles response, got: {other:?}"),
    }

    test_progress(
        "capability_endpoint_role_attestation_proof_paths",
        "tampered signature rejection",
    );
    // Tampering with the signature must fail during attestation verification.
    let mut issued = issue_self_attestation_as(&pic, root_id, signer_id, 60, Some(root_id));
    let issued_at = issued.payload.issued_at;
    if let Some(first) = issued.signature.first_mut() {
        *first ^= 0x01;
    }
    let envelope =
        cycles_role_attestation_envelope(root_id, request.clone(), issued, issued_at, 6, 4);
    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        signer_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let err = response.expect_err("tampered attestation signature must fail");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("signature"),
        "expected signature error, got: {err:?}"
    );

    test_progress(
        "capability_endpoint_role_attestation_proof_paths",
        "capability hash mismatch rejection",
    );
    // Capability hashes must match the request exactly.
    let issued = issue_self_attestation_as(&pic, root_id, signer_id, 60, Some(root_id));
    let issued_at = issued.payload.issued_at;
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request.clone(),
        proof: encode_role_attestation_capability_proof(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: [0u8; 32],
            attestation: issued,
        }),
        metadata: capability_metadata(issued_at, 9, 1, 60),
    };
    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        signer_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let err = response.expect_err("hash mismatch must fail closed");
    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(
        err.message.contains("capability_hash"),
        "expected capability_hash mismatch error, got: {err:?}"
    );

    test_progress(
        "capability_endpoint_role_attestation_proof_paths",
        "audience mismatch rejection",
    );
    // Audience mismatches must be enforced by the capability verifier.
    let wrong_audience = Principal::from_slice(&[9; 29]);
    let issued = issue_self_attestation_as(&pic, root_id, signer_id, 60, Some(wrong_audience));
    let issued_at = issued.payload.issued_at;
    let envelope =
        cycles_role_attestation_envelope(root_id, request.clone(), issued, issued_at, 3, 7);
    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        signer_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let err = response.expect_err("audience mismatch must fail");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("audience mismatch"),
        "expected audience mismatch error, got: {err:?}"
    );

    test_progress(
        "capability_endpoint_role_attestation_proof_paths",
        "expiry rejection",
    );
    // Expiry is time-sensitive, so keep it last after advancing the clock.
    let issued = issue_self_attestation_as(&pic, root_id, signer_id, 1, Some(root_id));
    let issued_at = issued.payload.issued_at;
    pic.advance_time(Duration::from_secs(2));
    pic.tick();
    let envelope = cycles_role_attestation_envelope(root_id, request, issued, issued_at, 2, 8);
    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        signer_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let err = response.expect_err("expired attestation must fail");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("expired"),
        "expected expired attestation error, got: {err:?}"
    );
    test_progress("capability_endpoint_role_attestation_proof_paths", "done");
}

#[test]
#[expect(clippy::too_many_lines)]
fn capability_endpoint_policy_and_structural_paths() {
    test_progress(
        "capability_endpoint_policy_and_structural_paths",
        "setup root",
    );
    let setup = install_test_root_cached();
    let pic = PicBorrow(setup.pic.pic());
    let root_id = setup.root_id;
    let signer_id = setup.signer_id;
    let issued = issue_self_attestation(&pic, root_id, 60, Some(root_id));
    let issued_at = issued.payload.issued_at;

    test_progress(
        "capability_endpoint_policy_and_structural_paths",
        "subject mismatch policy rejection",
    );
    // Policy must reject subject-mismatch requests even with a valid proof.
    let subject_mismatch_request = Request::IssueRoleAttestation(RoleAttestationRequest {
        subject: Principal::anonymous(),
        role: CanisterRole::ROOT,
        subnet_id: None,
        audience: Some(root_id),
        ttl_secs: 60,
        epoch: 0,
        metadata: None,
    });
    let subject_mismatch_hash = root_capability_hash(root_id, &subject_mismatch_request);
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: subject_mismatch_request.clone(),
        proof: encode_role_attestation_capability_proof(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: subject_mismatch_hash,
            attestation: issued.clone(),
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

    test_progress(
        "capability_endpoint_policy_and_structural_paths",
        "policy denial replay behavior",
    );
    // Policy denials must not poison replay detection for the same request id.
    let envelope_a = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: subject_mismatch_request.clone(),
        proof: encode_role_attestation_capability_proof(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: subject_mismatch_hash,
            attestation: issued.clone(),
        }),
        metadata: capability_metadata(issued_at, 4, 66, 60),
    };
    let envelope_b = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: subject_mismatch_request,
        proof: encode_role_attestation_capability_proof(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: subject_mismatch_hash,
            attestation: issued.clone(),
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

    test_progress(
        "capability_endpoint_policy_and_structural_paths",
        "missing audience rejection",
    );
    // Missing audiences must be rejected by the policy layer.
    let missing_audience_request = Request::IssueRoleAttestation(RoleAttestationRequest {
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
        capability: missing_audience_request.clone(),
        proof: encode_role_attestation_capability_proof(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: root_capability_hash(root_id, &missing_audience_request),
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

    test_progress(
        "capability_endpoint_policy_and_structural_paths",
        "supported structural proof",
    );
    // Structural proof is allowed only for the limited cycles family.
    let cycles_request = Request::Cycles(CyclesRequest {
        cycles: 1,
        metadata: None,
    });
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: cycles_request.clone(),
        proof: CapabilityProof::Structural,
        metadata: capability_metadata(issued_at, 7, 3, 60),
    };
    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        &pic,
        root_id,
        signer_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let response = response.expect("structural cycles proof should succeed");
    match response.response {
        Response::Cycles(res) => assert_eq!(res.cycles_transferred, 1),
        other => panic!("expected cycles response, got: {other:?}"),
    }

    test_progress(
        "capability_endpoint_policy_and_structural_paths",
        "unsupported structural rejection",
    );
    let unsupported_structural_request = Request::IssueRoleAttestation(RoleAttestationRequest {
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
        capability: unsupported_structural_request,
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

    test_progress(
        "capability_endpoint_policy_and_structural_paths",
        "delegated grant scope rejection",
    );
    // Delegated grants must name the correct capability family.
    let capability_hash = root_capability_hash(root_id, &cycles_request);
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: cycles_request,
        proof: encode_delegated_grant_capability_proof(DelegatedGrantProof {
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
    test_progress("capability_endpoint_policy_and_structural_paths", "done");
}

///
/// DelegationAdminFixture
///

struct DelegationAdminFixture {
    setup: CachedInstalledRoot,
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
fn delegation_admin_fixture(_subject_seed: u8) -> DelegationAdminFixture {
    let setup = install_test_root_with_verifier_cached();
    let root_id = setup.root_id;
    let signer_id = setup.signer_id;
    let verifier_id = setup.verifier_id.expect("cached verifier must exist");
    let cached = delegation_admin_cached_data(setup.pic.pic(), root_id, signer_id, verifier_id);

    DelegationAdminFixture {
        setup,
        root_id,
        signer_id,
        verifier_id,
        delegated_subject: cached.delegated_subject,
        stale_token: cached.stale_token,
        current_token: cached.current_token,
        root_public_key: cached.root_public_key,
        shard_public_key: cached.shard_public_key,
    }
}

// Reuse the same issued admin tokens and public keys across restored verifier baselines.
fn delegation_admin_cached_data(
    pic: &Pic,
    root_id: Principal,
    signer_id: Principal,
    verifier_id: Principal,
) -> DelegationAdminCachedData {
    let cache = DELEGATION_ADMIN_FIXTURE_CACHE.get_or_init(|| Mutex::new(None));
    let mut cache = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if let Some(cached) = cache.as_ref()
        && cached.root_id == root_id
        && cached.signer_id == signer_id
        && cached.verifier_id == verifier_id
    {
        return cached.clone();
    }

    let delegated_subject = Principal::from_slice(&[83; 29]);
    let stale_token =
        issue_test_delegated_token(pic, root_id, signer_id, verifier_id, delegated_subject, 60);
    let current_token =
        issue_test_delegated_token(pic, root_id, signer_id, verifier_id, delegated_subject, 120);
    let (root_public_key, shard_public_key) = delegation_public_keys(pic, root_id);

    let generated = DelegationAdminCachedData {
        root_id,
        signer_id,
        verifier_id,
        delegated_subject,
        stale_token,
        current_token,
        root_public_key,
        shard_public_key,
    };
    *cache = Some(generated.clone());
    generated
}

// Issue a test delegated token for the requested verifier audience and TTL.
fn issue_test_delegated_token(
    pic: &Pic,
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
fn delegation_public_keys(pic: &Pic, root_id: Principal) -> (Vec<u8>, Vec<u8>) {
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
    pic: &Pic,
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
    pic: &Pic,
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
    pic: &Pic,
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
    pic: &Pic,
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
    pic: &Pic,
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
    pic: &Pic,
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

// Issue one self-attestation from the root test hook for the requested audience.
fn issue_self_attestation(
    pic: &Pic,
    root_id: Principal,
    ttl_secs: u64,
    audience: Option<Principal>,
) -> SignedRoleAttestation {
    issue_self_attestation_as(pic, root_id, root_id, ttl_secs, audience)
}

// Issue one self-attestation from the root test hook as an explicit caller.
fn issue_self_attestation_as(
    pic: &Pic,
    root_id: Principal,
    caller: Principal,
    ttl_secs: u64,
    audience: Option<Principal>,
) -> SignedRoleAttestation {
    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        pic,
        root_id,
        caller,
        "root_issue_self_attestation_test",
        (ttl_secs, audience, 0u64),
    );

    issued.expect("attestation issuance failed")
}

// Build a cycles capability envelope backed by a role-attestation proof.
fn cycles_role_attestation_envelope(
    root_id: Principal,
    request: Request,
    attestation: SignedRoleAttestation,
    issued_at: u64,
    request_id_seed: u8,
    nonce_seed: u8,
) -> RootCapabilityEnvelopeV1 {
    RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request.clone(),
        proof: encode_role_attestation_capability_proof(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: root_capability_hash(root_id, &request),
            attestation,
        }),
        metadata: capability_metadata(issued_at, request_id_seed, nonce_seed, 60),
    }
}

fn update_call_as<T, A>(
    pic: &Pic,
    canister_id: Principal,
    caller: Principal,
    method: &str,
    args: A,
) -> T
where
    T: candid::CandidType + DeserializeOwned,
    A: ArgumentEncoder,
{
    pic.update_call_as(canister_id, caller, method, args)
        .expect("update_call failed")
}

fn update_call_raw_as<A>(
    pic: &Pic,
    canister_id: Principal,
    caller: Principal,
    method: &str,
    args: A,
) -> Result<(), String>
where
    A: ArgumentEncoder,
{
    pic.update_call_as::<Result<(), Error>, _>(canister_id, caller, method, args)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn query_call_as<T, A>(
    pic: &Pic,
    canister_id: Principal,
    caller: Principal,
    method: &str,
    args: A,
) -> T
where
    T: candid::CandidType + DeserializeOwned,
    A: ArgumentEncoder,
{
    pic.query_call_as(canister_id, caller, method, args)
        .expect("query_call failed")
}

fn access_metric_count(pic: &Pic, canister_id: Principal, endpoint: &str, predicate: &str) -> u64 {
    let response: Result<Page<MetricEntry>, Error> = query_call_as(
        pic,
        canister_id,
        Principal::anonymous(),
        "canic_metrics",
        (
            MetricsKind::Access,
            PageRequest {
                limit: 10_000,
                offset: 0,
            },
        ),
    );
    let response = response.expect("query canic_metrics failed");
    response
        .entries
        .into_iter()
        .find_map(|entry| {
            if entry.labels.first().is_some_and(|label| label == endpoint)
                && entry.labels.get(2).is_some_and(|label| label == predicate)
            {
                Some(match entry.value {
                    MetricValue::Count(count) | MetricValue::CountAndU64 { count, .. } => count,
                    MetricValue::U128(_) => 0,
                })
            } else {
                None
            }
        })
        .unwrap_or(0)
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
    RpcApi::root_capability_hash(target_canister, CAPABILITY_VERSION_V1, capability)
        .expect("compute root capability hash")
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
