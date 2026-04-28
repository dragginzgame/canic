use crate::pic_role_attestation_support::*;
use std::time::Duration;

#[test]
fn delegation_admin_repair_updates_stale_verifier_proof_and_records_metrics() {
    test_progress(
        "delegation_admin_repair_updates_stale_verifier_proof_and_records_metrics",
        "setup fixture",
    );
    let fixture = delegation_admin_fixture(83);

    test_progress(
        "delegation_admin_repair_updates_stale_verifier_proof_and_records_metrics",
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
        "delegation_admin_repair_updates_stale_verifier_proof_and_records_metrics",
        "repair verifier",
    );
    let repair = repair_verifiers(
        fixture.setup.pic.pic(),
        fixture.root_id,
        fixture.current_token.proof.clone(),
        vec![fixture.verifier_id],
    )
    .expect("repair admin call must succeed");
    let DelegationAdminResponse::RepairedVerifiers { result } = repair;
    assert_eq!(result.results.len(), 1);
    let response = &result.results[0];
    assert_eq!(response.target, fixture.verifier_id);
    assert_eq!(response.status, DelegationProvisionStatus::Ok);
    assert!(
        response.error.is_none(),
        "unexpected repair error: {response:?}"
    );

    let verified_after_repair: Result<(), Error> = update_call_as(
        fixture.setup.pic.pic(),
        fixture.verifier_id,
        fixture.delegated_subject,
        "signer_verify_token",
        (fixture.current_token,),
    );
    verified_after_repair.expect("repair should update verifier proof");

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
                "delegation_install_fanout_bucket{intent=\"repair\",bucket=\"1\"}",
                1,
            ),
            (
                "delegation_push_attempt{role=\"verifier\",origin=\"repair\"}",
                1,
            ),
            (
                "delegation_push_success{role=\"verifier\",origin=\"repair\"}",
                1,
            ),
            ("delegation_push_complete{origin=\"repair\"}", 1),
        ],
    );
    assert_access_metrics(
        fixture.setup.pic.pic(),
        fixture.verifier_id,
        "auth_verifier",
        &[("token_rejected_proof_miss", 1)],
    );
    test_progress(
        "delegation_admin_repair_updates_stale_verifier_proof_and_records_metrics",
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
            intent: DelegationProofInstallIntent::Repair,
            root_public_key_sec1: None,
            shard_public_key_sec1: vec![1, 2, 3],
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
            "delegation_install_validation_failed{intent=\"repair\",stage=\"post_normalization\",reason=\"target_not_in_audience\"}",
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
    fixture.setup.pic.pic().advance_time(Duration::from_secs(1));
    fixture.setup.pic.pic().tick();
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
        aud: DelegationAudience::Any,
        iat: now,
        exp: now + 120,
        ext: None,
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
