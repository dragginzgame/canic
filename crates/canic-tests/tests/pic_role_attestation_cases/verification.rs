use crate::pic_role_attestation_support::*;

#[test]
#[expect(
    clippy::significant_drop_tightening,
    reason = "pic borrows the cached setup owner for the full test"
)]
fn role_attestation_verification_paths() {
    let setup = install_test_root_cached();
    let pic = setup.pic.pic();
    let root_id = setup.root_id;

    // Happy path should verify a freshly issued self-attestation.
    let issued = issue_self_attestation(pic, root_id, TEST_ROLE_ATTESTATION_TTL_NS, root_id);
    let verified: Result<(), Error> = pic.update_call_as_or_panic(
        root_id,
        root_id,
        "root_verify_role_attestation",
        (issued, 0u64),
    );
    verified.expect("attestation verification failed");

    // Mismatched caller must fail even with an otherwise valid attestation.
    let issued = issue_self_attestation(pic, root_id, TEST_ROLE_ATTESTATION_TTL_NS, root_id);
    let verified: Result<(), Error> = pic.update_call_as_or_panic(
        root_id,
        Principal::anonymous(),
        "root_verify_role_attestation",
        (issued, 0u64),
    );
    let err = verified.expect_err("verification must fail for mismatched caller");
    assert_eq!(err.code, ErrorCode::Internal);

    // Audience binding must be enforced by the verifier.
    let wrong_audience = Principal::from_slice(&[9; 29]);
    let issued = issue_self_attestation(pic, root_id, TEST_ROLE_ATTESTATION_TTL_NS, wrong_audience);
    let verified: Result<(), Error> = pic.update_call_as_or_panic(
        root_id,
        root_id,
        "root_verify_role_attestation",
        (issued, 0u64),
    );
    let err = verified.expect_err("verification must fail for audience mismatch");
    assert_eq!(err.code, ErrorCode::Internal);

    // Epoch floors higher than the attestation epoch must fail closed.
    let issued = issue_self_attestation(pic, root_id, TEST_ROLE_ATTESTATION_TTL_NS, root_id);
    let verified: Result<(), Error> = pic.update_call_as_or_panic(
        root_id,
        root_id,
        "root_verify_role_attestation",
        (issued, 1u64),
    );
    let err = verified.expect_err("verification must fail when epoch floor is higher");
    assert_eq!(err.code, ErrorCode::Internal);

    // Expiry is time-sensitive, so keep it last after advancing the clock.
    let issued = issue_self_attestation(pic, root_id, TEST_SHORT_ROLE_ATTESTATION_TTL_NS, root_id);
    pic.advance_time(Duration::from_secs(2));
    pic.tick();
    let verified: Result<(), Error> = pic.update_call_as_or_panic(
        root_id,
        root_id,
        "root_verify_role_attestation",
        (issued, 0u64),
    );
    let err = verified.expect_err("verification must fail for expired attestation");
    assert_eq!(err.code, ErrorCode::Internal);
}
