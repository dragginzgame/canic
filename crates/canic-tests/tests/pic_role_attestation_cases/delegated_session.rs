use crate::pic_role_attestation_support::*;

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
        aud: DelegationAudience::Any,
        iat: now,
        exp: now + 120,
        ext: None,
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
        aud: DelegationAudience::Any,
        iat: now,
        exp: now + 120,
        ext: None,
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
        aud: DelegationAudience::Any,
        iat: now,
        exp: now + 120,
        ext: None,
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
        aud: DelegationAudience::Any,
        iat: now,
        exp: now + 120,
        ext: None,
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
        aud: DelegationAudience::Any,
        iat: now,
        exp: now + 180,
        ext: None,
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
        aud: DelegationAudience::Any,
        iat: now,
        exp: now + 240,
        ext: None,
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
        aud: DelegationAudience::Any,
        iat: now,
        exp: now + 5,
        ext: None,
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
