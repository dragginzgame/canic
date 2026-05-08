use crate::pic_role_attestation_support::*;

#[test]
fn role_attestation_verification_paths() {
    let setup = install_test_root_cached();
    let pic = PicBorrow(setup.pic.pic());
    let root_id = setup.root_id;

    // Happy path should verify a freshly issued self-attestation.
    let issued = issue_self_attestation(&pic, root_id, 60, root_id);
    let verified: Result<(), Error> = update_call_as(
        &pic,
        root_id,
        root_id,
        "root_verify_role_attestation",
        (issued, 0u64),
    );
    verified.expect("attestation verification failed");

    // Mismatched caller must fail even with an otherwise valid attestation.
    let issued = issue_self_attestation(&pic, root_id, 60, root_id);
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
    let issued = issue_self_attestation(&pic, root_id, 60, wrong_audience);
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
    let issued = issue_self_attestation(&pic, root_id, 60, root_id);
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
    let issued = issue_self_attestation(&pic, root_id, 1, root_id);
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
        (60u64, root_id, 0u64, previous_key_id, previous_key_seed),
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
        (60u64, root_id, 0u64, current_key_id, current_key_seed),
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
