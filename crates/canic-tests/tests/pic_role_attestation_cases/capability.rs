use crate::pic_role_attestation_support::*;

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
    let issued = issue_self_attestation_as(&pic, root_id, signer_id, 60, root_id);
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
    let mut issued = issue_self_attestation_as(&pic, root_id, signer_id, 60, root_id);
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
    let issued = issue_self_attestation_as(&pic, root_id, signer_id, 60, root_id);
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
    let issued = issue_self_attestation_as(&pic, root_id, signer_id, 60, wrong_audience);
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
    let issued = issue_self_attestation_as(&pic, root_id, signer_id, 1, root_id);
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
    let issued = issue_self_attestation(&pic, root_id, 60, root_id);
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
        audience: root_id,
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
        audience: root_id,
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
