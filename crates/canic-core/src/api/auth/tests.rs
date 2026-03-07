use super::*;
use crate::InternalErrorOrigin;
use crate::ops::auth::{DelegatedTokenOpsError, DelegationExpiryError, DelegationValidationError};
use futures::executor::block_on;
use std::cell::Cell;

#[test]
fn verify_role_attestation_with_single_refresh_accepts_without_refresh() {
    let verify_calls = Cell::new(0usize);
    let refresh_calls = Cell::new(0usize);

    let result = block_on(verify_flow::verify_role_attestation_with_single_refresh(
        || {
            verify_calls.set(verify_calls.get() + 1);
            Ok(())
        },
        || {
            refresh_calls.set(refresh_calls.get() + 1);
            std::future::ready(Ok(()))
        },
    ));

    assert!(result.is_ok());
    assert_eq!(verify_calls.get(), 1, "verify must run exactly once");
    assert_eq!(refresh_calls.get(), 0, "refresh must not run");
}

#[test]
fn verify_role_attestation_with_single_refresh_retries_once_on_unknown_key() {
    let verify_calls = Cell::new(0usize);
    let refresh_calls = Cell::new(0usize);

    let result = block_on(verify_flow::verify_role_attestation_with_single_refresh(
        || {
            let attempt = verify_calls.get();
            verify_calls.set(attempt + 1);
            if attempt == 0 {
                Err(DelegationValidationError::AttestationUnknownKeyId { key_id: 7 }.into())
            } else {
                Ok(())
            }
        },
        || {
            refresh_calls.set(refresh_calls.get() + 1);
            std::future::ready(Ok(()))
        },
    ));

    assert!(result.is_ok());
    assert_eq!(verify_calls.get(), 2, "verify must run exactly twice");
    assert_eq!(refresh_calls.get(), 1, "refresh must run exactly once");
}

#[test]
fn verify_role_attestation_with_single_refresh_fails_closed_on_refresh_error() {
    let verify_calls = Cell::new(0usize);
    let refresh_calls = Cell::new(0usize);

    let result = block_on(verify_flow::verify_role_attestation_with_single_refresh(
        || {
            verify_calls.set(verify_calls.get() + 1);
            Err(DelegationValidationError::AttestationUnknownKeyId { key_id: 9 }.into())
        },
        || {
            refresh_calls.set(refresh_calls.get() + 1);
            std::future::ready(Err(crate::InternalError::infra(
                InternalErrorOrigin::Infra,
                "refresh failed",
            )))
        },
    ));

    match result {
        Err(verify_flow::RoleAttestationVerifyFlowError::Refresh {
            trigger:
                DelegatedTokenOpsError::Validation(DelegationValidationError::AttestationUnknownKeyId {
                    key_id,
                }),
            ..
        }) => assert_eq!(key_id, 9),
        other => panic!("expected refresh failure for unknown key, got: {other:?}"),
    }

    assert_eq!(
        verify_calls.get(),
        1,
        "verify must not retry after refresh failure"
    );
    assert_eq!(refresh_calls.get(), 1, "refresh must run once");
}

#[test]
fn verify_role_attestation_with_single_refresh_does_not_refresh_on_non_unknown_error() {
    let verify_calls = Cell::new(0usize);
    let refresh_calls = Cell::new(0usize);

    let result = block_on(verify_flow::verify_role_attestation_with_single_refresh(
        || {
            verify_calls.set(verify_calls.get() + 1);
            Err(DelegationExpiryError::AttestationEpochRejected {
                epoch: 1,
                min_accepted_epoch: 2,
            }
            .into())
        },
        || {
            refresh_calls.set(refresh_calls.get() + 1);
            std::future::ready(Ok(()))
        },
    ));

    match result {
        Err(verify_flow::RoleAttestationVerifyFlowError::Initial(
            DelegatedTokenOpsError::Expiry(DelegationExpiryError::AttestationEpochRejected {
                epoch,
                min_accepted_epoch,
            }),
        )) => {
            assert_eq!(epoch, 1);
            assert_eq!(min_accepted_epoch, 2);
        }
        other => panic!("expected initial epoch rejection, got: {other:?}"),
    }

    assert_eq!(verify_calls.get(), 1, "verify must run once");
    assert_eq!(refresh_calls.get(), 0, "refresh must not run");
}

#[test]
fn verify_role_attestation_with_single_refresh_only_attempts_one_refresh() {
    let verify_calls = Cell::new(0usize);
    let refresh_calls = Cell::new(0usize);

    let result = block_on(verify_flow::verify_role_attestation_with_single_refresh(
        || {
            let attempt = verify_calls.get();
            verify_calls.set(attempt + 1);
            if attempt == 0 {
                Err(DelegationValidationError::AttestationUnknownKeyId { key_id: 5 }.into())
            } else {
                Err(DelegationValidationError::AttestationUnknownKeyId { key_id: 6 }.into())
            }
        },
        || {
            refresh_calls.set(refresh_calls.get() + 1);
            std::future::ready(Ok(()))
        },
    ));

    match result {
        Err(verify_flow::RoleAttestationVerifyFlowError::PostRefresh(
            DelegatedTokenOpsError::Validation(
                DelegationValidationError::AttestationUnknownKeyId { key_id },
            ),
        )) => assert_eq!(key_id, 6),
        other => panic!("expected post-refresh unknown-key rejection, got: {other:?}"),
    }

    assert_eq!(verify_calls.get(), 2, "verify must run exactly twice");
    assert_eq!(refresh_calls.get(), 1, "refresh must run exactly once");
}

#[test]
fn resolve_min_accepted_epoch_prefers_explicit_argument() {
    assert_eq!(verify_flow::resolve_min_accepted_epoch(7, Some(3)), 7);
    assert_eq!(verify_flow::resolve_min_accepted_epoch(5, None), 5);
}

#[test]
fn resolve_min_accepted_epoch_falls_back_to_config_or_zero() {
    assert_eq!(verify_flow::resolve_min_accepted_epoch(0, Some(4)), 4);
    assert_eq!(verify_flow::resolve_min_accepted_epoch(0, None), 0);
}
