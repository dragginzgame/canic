use crate::pic_role_attestation_support::*;

#[test]
fn capability_endpoint_rejects_role_attestation_proofs_after_hard_cut() {
    test_progress(
        "capability_endpoint_rejects_role_attestation_proofs_after_hard_cut",
        "setup root",
    );
    let setup = install_test_root_cached();
    let pic = setup.pic.pic();
    let root_id = setup.root_id;
    let signer_id = setup.signer_id;
    let request = Request::Cycles(CyclesRequest {
        cycles: 1,
        metadata: None,
    });

    test_progress(
        "capability_endpoint_rejects_role_attestation_proofs_after_hard_cut",
        "disabled proof rejection",
    );
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request.clone(),
        proof: CapabilityProof::RoleAttestation(CapabilityProofBlob {
            proof_version: 1,
            capability_hash: root_capability_hash(root_id, &request),
            payload: Vec::new(),
        }),
        metadata: capability_metadata(0, 9, 1, TEST_ROLE_ATTESTATION_TTL_NS),
    };
    let response: Result<RootCapabilityResponseV1, Error> = pic.update_call_as_or_panic(
        root_id,
        signer_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let err = response.expect_err("role-attestation capability proof must fail closed");
    assert_eq!(err.code, ErrorCode::Forbidden);
    assert!(
        err.message.contains("disabled in 0.65"),
        "expected hard-cut rejection, got: {err:?}"
    );
    test_progress(
        "capability_endpoint_rejects_role_attestation_proofs_after_hard_cut",
        "done",
    );
}

#[test]
#[expect(clippy::too_many_lines)]
fn capability_endpoint_policy_and_structural_paths() {
    test_progress(
        "capability_endpoint_policy_and_structural_paths",
        "setup root",
    );
    let setup = install_test_root_cached();
    let pic = setup.pic.pic();
    let root_id = setup.root_id;
    let signer_id = setup.signer_id;
    let issued_at_ns = pic.current_time_nanos();

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
        metadata: capability_metadata(issued_at_ns, 7, 3, TEST_ROLE_ATTESTATION_TTL_NS),
    };
    let response: Result<RootCapabilityResponseV1, Error> = pic.update_call_as_or_panic(
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
        ttl_ns: TEST_ROLE_ATTESTATION_TTL_NS,
        epoch: 0,
        metadata: None,
    });
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: unsupported_structural_request,
        proof: CapabilityProof::Structural,
        metadata: capability_metadata(issued_at_ns, 7, 3, TEST_ROLE_ATTESTATION_TTL_NS),
    };
    let response: Result<RootCapabilityResponseV1, Error> = pic.update_call_as_or_panic(
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
                issued_at_ns,
                expires_at_ns: issued_at_ns.saturating_add(TEST_ROLE_ATTESTATION_TTL_NS),
                epoch: 0,
            },
            grant_sig: vec![1, 2, 3],
            key_id: 1,
        }),
        metadata: capability_metadata(issued_at_ns, 8, 2, TEST_ROLE_ATTESTATION_TTL_NS),
    };
    let response: Result<RootCapabilityResponseV1, Error> = pic.update_call_as_or_panic(
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
