use super::*;
use crate::storage::stable::authority_restore::{
    AuthorityRestoreFenceData, AuthorityRestoreFenceStore,
};

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte])
}

fn reset() {
    AuthorityRestoreFenceStore::import(AuthorityRestoreFenceData::default());
}

#[test]
fn restored_sealed_authority_cannot_resume_after_history_advances() {
    reset();
    let authority = principal(1);
    AuthorityRestoreFenceOps::initialize(authority).expect("initialize");
    let request = AuthoritySnapshotRequest {
        operation_id: [7; 32],
    };
    let sealed = AuthorityRestoreFenceOps::prepare(request, authority, 11, 13).expect("seal");
    assert_eq!(sealed.phase, AuthorityRestoreFencePhase::Sealed);

    let error = AuthorityRestoreFenceOps::resume(request, authority, 12, 17)
        .expect_err("advanced history must remain fenced");
    assert_eq!(
        error.public_error().code(),
        crate::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
    );
    assert_eq!(
        AuthorityRestoreFenceOps::status().expect("status").phase,
        AuthorityRestoreFencePhase::Sealed
    );
}

#[test]
fn unchanged_live_history_resumes_once_and_replays_exactly() {
    reset();
    let authority = principal(2);
    AuthorityRestoreFenceOps::initialize(authority).expect("initialize");
    let request = AuthoritySnapshotRequest {
        operation_id: [19; 32],
    };
    AuthorityRestoreFenceOps::prepare(request, authority, 23, 29).expect("seal");

    let resumed = AuthorityRestoreFenceOps::resume(request, authority, 23, 31).expect("resume");
    assert_eq!(resumed.phase, AuthorityRestoreFencePhase::Open);
    assert_eq!(
        AuthorityRestoreFenceOps::resume(request, authority, 23, 37).expect("exact replay"),
        resumed
    );
    assert!(
        !AuthorityRestoreFenceOps::is_sealed_for(authority).expect("live resumed authority state")
    );
}

#[test]
fn sealed_prepare_replays_exactly_and_rejects_another_operation() {
    reset();
    let authority = principal(8);
    AuthorityRestoreFenceOps::initialize(authority).expect("initialize");
    let request = AuthoritySnapshotRequest {
        operation_id: [53; 32],
    };
    let sealed = AuthorityRestoreFenceOps::prepare(request, authority, 59, 61).expect("seal");
    assert_eq!(
        AuthorityRestoreFenceOps::prepare(request, authority, 67, 71).expect("exact retry"),
        sealed
    );

    let conflicting = AuthoritySnapshotRequest {
        operation_id: [73; 32],
    };
    assert_eq!(
        AuthorityRestoreFenceOps::prepare(conflicting, authority, 59, 61)
            .expect_err("different prepare operation must fail")
            .public_error()
            .code(),
        crate::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(
        AuthorityRestoreFenceOps::resume(conflicting, authority, 59, 61)
            .expect_err("different resume operation must fail")
            .public_error()
            .code(),
        crate::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    assert_eq!(AuthorityRestoreFenceOps::status().expect("status"), sealed);
}

#[test]
fn sealed_authority_exposes_validated_state() {
    reset();
    let authority = principal(3);
    AuthorityRestoreFenceOps::initialize(authority).expect("initialize");
    AuthorityRestoreFenceOps::prepare(
        AuthoritySnapshotRequest {
            operation_id: [41; 32],
        },
        authority,
        43,
        47,
    )
    .expect("seal");

    assert!(AuthorityRestoreFenceOps::is_sealed_for(authority).expect("sealed state"));
}

#[test]
fn validation_phases_do_not_commit_fence_transitions() {
    reset();
    let authority = principal(9);
    let request = AuthoritySnapshotRequest {
        operation_id: [83; 32],
    };
    AuthorityRestoreFenceOps::initialize(authority).expect("initialize");

    AuthorityRestoreFenceOps::validate_prepare(request, authority).expect("validate prepare");
    assert_eq!(
        AuthorityRestoreFenceOps::status()
            .expect("open status")
            .phase,
        AuthorityRestoreFencePhase::Open
    );

    AuthorityRestoreFenceOps::prepare(request, authority, 89, 97).expect("seal");
    AuthorityRestoreFenceOps::validate_resume(request, authority, 89).expect("validate resume");
    assert_eq!(
        AuthorityRestoreFenceOps::status()
            .expect("still sealed status")
            .phase,
        AuthorityRestoreFencePhase::Sealed
    );
}

#[test]
fn snapshot_seal_requires_a_nonzero_operation_identity() {
    reset();
    let authority = principal(7);
    AuthorityRestoreFenceOps::initialize(authority).expect("initialize");
    let error = AuthorityRestoreFenceOps::prepare(
        AuthoritySnapshotRequest {
            operation_id: [0; 32],
        },
        authority,
        11,
        13,
    )
    .expect_err("zero operation identity must fail");
    assert_eq!(
        error.public_error().code(),
        crate::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
    );
}

#[test]
fn missing_or_mismatched_authority_fails_closed() {
    reset();
    let missing = AuthorityRestoreFenceOps::is_sealed_for(principal(4)).expect_err("missing state");
    assert_eq!(missing.code(), crate::diagnostics::codes::STATE_INVALID);

    AuthorityRestoreFenceOps::initialize(principal(5)).expect("initialize");
    let mismatched =
        AuthorityRestoreFenceOps::is_sealed_for(principal(6)).expect_err("mismatched authority");
    assert_eq!(
        mismatched.public_error().code(),
        crate::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
}

fn release_request() -> AuthorityReleaseRequest {
    AuthorityReleaseRequest {
        operation_id: [7; 32],
        review_sha256: [8; 32],
        recipient: principal(9),
    }
}

#[test]
fn release_seal_reopens_exactly_and_cannot_become_a_snapshot() {
    reset();
    let authority = principal(1);
    let request = release_request();
    AuthorityRestoreFenceOps::initialize(authority).unwrap();
    AuthorityRestoreFenceOps::validate_release(request, authority).unwrap();
    assert_eq!(
        AuthorityRestoreFenceOps::mutation_fence_for(authority).unwrap(),
        AuthorityMutationFence::Open
    );
    let sealed = AuthorityRestoreFenceOps::seal_release(request, authority, 11).unwrap();
    assert_eq!(
        sealed.phase,
        AuthorityRestoreFencePhase::ReleaseSealed {
            review_sha256: request.review_sha256,
            recipient: request.recipient
        }
    );
    assert_eq!(sealed.operation_id, Some(request.operation_id));
    assert_eq!(sealed.history_total_num_changes, None);
    let retained = AuthorityRestoreFenceStore::export();
    reset();
    AuthorityRestoreFenceStore::import(retained.clone());
    assert_eq!(
        AuthorityRestoreFenceOps::seal_release(request, authority, 99).unwrap(),
        sealed
    );
    AuthorityRestoreFenceOps::validate_release(request, authority).unwrap();
    assert!(AuthorityRestoreFenceOps::is_sealed_for(authority).unwrap());
    let snapshot = AuthoritySnapshotRequest {
        operation_id: request.operation_id,
    };
    for error in [
        AuthorityRestoreFenceOps::validate_prepare(snapshot, authority).unwrap_err(),
        AuthorityRestoreFenceOps::prepare(snapshot, authority, 1, 99).unwrap_err(),
        AuthorityRestoreFenceOps::validate_resume(snapshot, authority, 1).unwrap_err(),
        AuthorityRestoreFenceOps::resume(snapshot, authority, 1, 99).unwrap_err(),
    ] {
        assert_eq!(error.code(), crate::diagnostics::codes::STATE_CONFLICT);
    }
    assert_eq!(AuthorityRestoreFenceStore::export(), retained);
}

#[test]
fn release_seal_never_rebinds_identity_or_overwrites_snapshot_authority() {
    reset();
    let authority = principal(1);
    let request = release_request();
    AuthorityRestoreFenceOps::initialize(authority).unwrap();
    AuthorityRestoreFenceOps::seal_release(request, authority, 11).unwrap();
    let retained = AuthorityRestoreFenceStore::export();
    for change in [
        |request: &mut AuthorityReleaseRequest| request.operation_id = [10; 32],
        |request: &mut AuthorityReleaseRequest| request.review_sha256 = [10; 32],
        |request: &mut AuthorityReleaseRequest| request.recipient = principal(10),
    ] {
        let mut changed = request;
        change(&mut changed);
        for error in [
            AuthorityRestoreFenceOps::validate_release(changed, authority).unwrap_err(),
            AuthorityRestoreFenceOps::seal_release(changed, authority, 19).unwrap_err(),
        ] {
            assert_eq!(error.code(), crate::diagnostics::codes::STATE_CONFLICT);
        }
        assert_eq!(AuthorityRestoreFenceStore::export(), retained);
    }
    assert_eq!(
        AuthorityRestoreFenceOps::validate_release(request, principal(2))
            .unwrap_err()
            .code(),
        crate::diagnostics::codes::STATE_CONFLICT
    );
    reset();
    AuthorityRestoreFenceOps::initialize(authority).unwrap();
    AuthorityRestoreFenceOps::prepare(
        AuthoritySnapshotRequest {
            operation_id: request.operation_id,
        },
        authority,
        1,
        11,
    )
    .unwrap();
    let snapshot = AuthorityRestoreFenceStore::export();
    assert_eq!(
        AuthorityRestoreFenceOps::seal_release(request, authority, 19)
            .unwrap_err()
            .code(),
        crate::diagnostics::codes::STATE_CONFLICT
    );
    assert_eq!(AuthorityRestoreFenceStore::export(), snapshot);
}

#[test]
fn invalid_release_bindings_leave_the_open_fence_unchanged() {
    reset();
    let authority = principal(1);
    AuthorityRestoreFenceOps::initialize(authority).unwrap();
    let retained = AuthorityRestoreFenceStore::export();
    for recipient in [
        authority,
        Principal::anonymous(),
        Principal::management_canister(),
    ] {
        let request = AuthorityReleaseRequest {
            recipient,
            ..release_request()
        };
        assert_eq!(
            AuthorityRestoreFenceOps::seal_release(request, authority, 11)
                .unwrap_err()
                .code(),
            crate::diagnostics::codes::REQUEST_INVALID
        );
    }
    let empty_review = AuthorityReleaseRequest {
        review_sha256: [0; 32],
        ..release_request()
    };
    assert_eq!(
        AuthorityRestoreFenceOps::seal_release(empty_review, authority, 11)
            .unwrap_err()
            .code(),
        crate::diagnostics::codes::REQUEST_INVALID
    );
    let empty_operation = AuthorityReleaseRequest {
        operation_id: [0; 32],
        ..release_request()
    };
    assert_eq!(
        AuthorityRestoreFenceOps::seal_release(empty_operation, authority, 11)
            .unwrap_err()
            .code(),
        crate::diagnostics::codes::AUTHORITY_UNAVAILABLE
    );
    assert_eq!(AuthorityRestoreFenceStore::export(), retained);
}
