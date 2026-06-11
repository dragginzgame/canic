use crate::pic_role_attestation_support::*;

#[test]
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
    test_progress("capability_endpoint_policy_and_structural_paths", "done");
}
