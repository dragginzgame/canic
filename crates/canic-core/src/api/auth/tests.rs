use super::*;
use crate::InternalErrorOrigin;
use crate::cdk::types::Principal;
use crate::dto::auth::{
    DelegatedTokenClaims, DelegationCert, DelegationProof, DelegationProvisionResponse,
    DelegationProvisionStatus, DelegationProvisionTargetKind, DelegationProvisionTargetResponse,
};
use crate::dto::error::ErrorCode;
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

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn sample_claims() -> DelegatedTokenClaims {
    DelegatedTokenClaims {
        sub: p(9),
        shard_pid: p(2),
        scopes: vec!["verify".to_string()],
        aud: vec![p(3)],
        iat: 100,
        exp: 120,
    }
}

fn sample_proof() -> DelegationProof {
    DelegationProof {
        cert: DelegationCert {
            root_pid: p(1),
            shard_pid: p(2),
            issued_at: 90,
            expires_at: 130,
            scopes: vec!["verify".to_string(), "read".to_string()],
            aud: vec![p(3), p(4)],
        },
        cert_sig: vec![1, 2, 3],
    }
}

#[test]
fn proof_is_reusable_for_claims_accepts_valid_subset_and_time_window() {
    let claims = sample_claims();
    let proof = sample_proof();
    assert!(DelegationApi::proof_is_reusable_for_claims(
        &proof, &claims, 110
    ));
}

#[test]
fn proof_is_reusable_for_claims_rejects_expired_cert() {
    let claims = sample_claims();
    let proof = sample_proof();
    assert!(!DelegationApi::proof_is_reusable_for_claims(
        &proof, &claims, 131
    ));
}

#[test]
fn proof_is_reusable_for_claims_rejects_scope_mismatch() {
    let mut claims = sample_claims();
    claims.scopes = vec!["admin".to_string()];
    let proof = sample_proof();
    assert!(!DelegationApi::proof_is_reusable_for_claims(
        &proof, &claims, 110
    ));
}

#[test]
fn derive_required_verifier_targets_excludes_root_and_signer_and_dedupes() {
    let signer_pid = p(1);
    let root_pid = p(2);
    let verifier_a = p(3);
    let verifier_b = p(4);
    let audience = vec![signer_pid, root_pid, verifier_a, verifier_a, verifier_b];

    let derived = DelegationApi::derive_required_verifier_targets_from_aud(
        &audience,
        signer_pid,
        root_pid,
        |principal| principal == verifier_a || principal == verifier_b,
    )
    .expect("target derivation should succeed");

    assert_eq!(derived, vec![verifier_a, verifier_b]);
}

#[test]
fn derive_required_verifier_targets_rejects_invalid_audience_target() {
    let signer_pid = p(1);
    let root_pid = p(2);
    let invalid_verifier = p(9);
    let audience = vec![invalid_verifier];

    let err = DelegationApi::derive_required_verifier_targets_from_aud(
        &audience,
        signer_pid,
        root_pid,
        |_principal| false,
    )
    .expect_err("invalid verifier target must fail closed");

    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(
        err.message
            .contains("invalid for canonical verifier provisioning"),
        "expected strict invalid-target message, got: {err:?}"
    );
}

#[test]
fn signer_issuance_fails_when_required_verifier_proof_missing_regression() {
    let required_target = p(7);
    let response = DelegationProvisionResponse {
        proof: sample_proof(),
        results: vec![],
    };

    let err =
        DelegationApi::ensure_required_verifier_targets_provisioned(&[required_target], &response)
            .expect_err("missing required verifier proof fanout must fail issuance");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("missing verifier target result"),
        "expected missing-result message, got: {err:?}"
    );
}

#[test]
fn ensure_required_verifier_targets_provisioned_accepts_all_ok_results() {
    let required_target = p(7);
    let response = DelegationProvisionResponse {
        proof: sample_proof(),
        results: vec![DelegationProvisionTargetResponse {
            target: required_target,
            kind: DelegationProvisionTargetKind::Verifier,
            status: DelegationProvisionStatus::Ok,
            error: None,
        }],
    };

    DelegationApi::ensure_required_verifier_targets_provisioned(&[required_target], &response)
        .expect("required verifier fanout should pass when target is ok");
}

#[test]
fn ensure_required_verifier_targets_provisioned_rejects_failed_target() {
    let required_target = p(7);
    let response = DelegationProvisionResponse {
        proof: sample_proof(),
        results: vec![DelegationProvisionTargetResponse {
            target: required_target,
            kind: DelegationProvisionTargetKind::Verifier,
            status: DelegationProvisionStatus::Failed,
            error: Some(Error::internal("simulated push failure")),
        }],
    };

    let err =
        DelegationApi::ensure_required_verifier_targets_provisioned(&[required_target], &response)
            .expect_err("failed verifier fanout must fail closed");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("failed for required verifier target"),
        "expected provisioning failure message, got: {err:?}"
    );
}

#[test]
fn ensure_required_verifier_targets_provisioned_rejects_missing_target_result() {
    let required_target = p(7);
    let response = DelegationProvisionResponse {
        proof: sample_proof(),
        results: vec![DelegationProvisionTargetResponse {
            target: p(8),
            kind: DelegationProvisionTargetKind::Verifier,
            status: DelegationProvisionStatus::Ok,
            error: None,
        }],
    };

    let err =
        DelegationApi::ensure_required_verifier_targets_provisioned(&[required_target], &response)
            .expect_err("missing verifier fanout result must fail closed");
    assert_eq!(err.code, ErrorCode::Internal);
    assert!(
        err.message.contains("missing verifier target result"),
        "expected missing-result message, got: {err:?}"
    );
}

#[test]
fn clamp_delegated_session_expires_at_clamps_to_token_expiry() {
    let expires_at = DelegationApi::clamp_delegated_session_expires_at(100, 130, 600, Some(500))
        .expect("clamp should succeed");
    assert_eq!(expires_at, 130);
}

#[test]
fn clamp_delegated_session_expires_at_clamps_to_configured_max_ttl() {
    let expires_at = DelegationApi::clamp_delegated_session_expires_at(100, 900, 60, Some(500))
        .expect("clamp should succeed");
    assert_eq!(expires_at, 160);
}

#[test]
fn clamp_delegated_session_expires_at_clamps_to_requested_ttl() {
    let expires_at = DelegationApi::clamp_delegated_session_expires_at(100, 900, 600, Some(30))
        .expect("clamp should succeed");
    assert_eq!(expires_at, 130);
}

#[test]
fn clamp_delegated_session_expires_at_rejects_zero_requested_ttl() {
    let err = DelegationApi::clamp_delegated_session_expires_at(100, 900, 600, Some(0))
        .expect_err("zero requested ttl must fail");
    assert_eq!(err.code, crate::dto::error::ErrorCode::InvalidInput);
}

#[test]
fn clamp_delegated_session_expires_at_rejects_expired_token() {
    let err = DelegationApi::clamp_delegated_session_expires_at(100, 100, 600, Some(30))
        .expect_err("expired token must fail");
    assert_eq!(err.code, crate::dto::error::ErrorCode::Forbidden);
}
