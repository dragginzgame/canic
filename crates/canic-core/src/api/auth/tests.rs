use super::proof_store::AudienceBindingFailureStage;
use super::*;
use crate::InternalErrorOrigin;
use crate::cdk::types::Principal;
use crate::dto::auth::{
    DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof,
    DelegationProofInstallIntent, DelegationProvisionResponse, DelegationProvisionStatus,
    DelegationProvisionTargetKind, DelegationProvisionTargetResponse,
    DelegationVerifierProofPushRequest,
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

fn sample_token() -> DelegatedToken {
    DelegatedToken {
        claims: sample_claims(),
        proof: sample_proof(),
        token_sig: vec![4, 5, 6],
    }
}

#[test]
fn proof_is_reusable_for_claims_accepts_valid_subset_and_time_window() {
    let claims = sample_claims();
    let proof = sample_proof();
    assert!(DelegatedTokenOps::proof_reusable_for_claims(
        &proof, &claims, 110
    ));
}

#[test]
fn proof_is_reusable_for_claims_rejects_expired_cert() {
    let claims = sample_claims();
    let proof = sample_proof();
    assert!(!DelegatedTokenOps::proof_reusable_for_claims(
        &proof, &claims, 131
    ));
}

#[test]
fn proof_is_reusable_for_claims_rejects_scope_mismatch() {
    let mut claims = sample_claims();
    claims.scopes = vec!["admin".to_string()];
    let proof = sample_proof();
    assert!(!DelegatedTokenOps::proof_reusable_for_claims(
        &proof, &claims, 110
    ));
}

#[test]
fn ensure_token_claim_audience_subset_accepts_subset() {
    let token = sample_token();

    DelegationApi::ensure_token_claim_audience_subset(&token)
        .expect("subset audience must be accepted");
}

#[test]
fn ensure_token_claim_audience_subset_uses_set_semantics() {
    let mut token = sample_token();
    token.claims.aud = vec![p(4), p(3), p(3)];

    DelegationApi::ensure_token_claim_audience_subset(&token)
        .expect("duplicate and reordered audience entries must be accepted");
}

#[test]
fn ensure_token_claim_audience_subset_rejects_empty_claim_audience() {
    let mut token = sample_token();
    token.claims.aud.clear();

    let err = DelegationApi::ensure_token_claim_audience_subset(&token)
        .expect_err("empty claims audience must fail");

    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(err.message.contains("must not be empty"));
}

#[test]
fn ensure_token_claim_audience_subset_rejects_claim_outside_proof_audience() {
    let mut token = sample_token();
    token.claims.aud.push(p(9));

    let err = DelegationApi::ensure_token_claim_audience_subset(&token)
        .expect_err("claims audience outside proof audience must fail");

    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(err.message.contains("is not a subset of proof audience"));
}

#[test]
fn ensure_token_claim_audience_subset_rejects_empty_proof_audience() {
    let mut token = sample_token();
    token.proof.cert.aud.clear();

    let err = DelegationApi::ensure_token_claim_audience_subset(&token)
        .expect_err("empty proof audience must fail");

    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(err.message.contains("is not a subset of proof audience"));
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
fn normalize_explicit_verifier_push_request_dedupes_targets() {
    let root_pid = p(1);
    let verifier_a = p(3);
    let verifier_b = p(4);

    let normalized = DelegationApi::normalize_explicit_verifier_push_request_with(
        DelegationVerifierProofPushRequest {
            proof: sample_proof(),
            verifier_targets: vec![verifier_a, verifier_a, verifier_b],
        },
        DelegationProofInstallIntent::Repair,
        root_pid,
        |principal| principal == verifier_a || principal == verifier_b,
    )
    .expect("normalization should succeed");

    assert_eq!(normalized.verifier_targets, vec![verifier_a, verifier_b]);
}

#[test]
fn normalize_explicit_verifier_push_request_rejects_signer_target() {
    let err = DelegationApi::normalize_explicit_verifier_push_request_with(
        DelegationVerifierProofPushRequest {
            proof: sample_proof(),
            verifier_targets: vec![sample_proof().cert.shard_pid],
        },
        DelegationProofInstallIntent::Repair,
        p(1),
        |_principal| true,
    )
    .expect_err("signer target must fail");

    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(err.message.contains("must not match signer shard"));
}

#[test]
fn normalize_explicit_verifier_push_request_rejects_root_target() {
    let root_pid = p(1);

    let err = DelegationApi::normalize_explicit_verifier_push_request_with(
        DelegationVerifierProofPushRequest {
            proof: sample_proof(),
            verifier_targets: vec![root_pid],
        },
        DelegationProofInstallIntent::Repair,
        root_pid,
        |_principal| true,
    )
    .expect_err("root target must fail");

    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(err.message.contains("must not match root canister"));
}

#[test]
fn normalize_explicit_verifier_push_request_rejects_unregistered_target() {
    let err = DelegationApi::normalize_explicit_verifier_push_request_with(
        DelegationVerifierProofPushRequest {
            proof: sample_proof(),
            verifier_targets: vec![p(99)],
        },
        DelegationProofInstallIntent::Repair,
        p(1),
        |_principal| false,
    )
    .expect_err("unregistered target must fail");

    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(err.message.contains("is not registered"));
}

#[test]
fn normalize_explicit_verifier_push_request_rejects_target_not_in_audience() {
    let root_pid = p(1);
    let verifier_a = p(3);
    let verifier_b = p(4);
    let out_of_audience = p(9);

    let err = DelegationApi::normalize_explicit_verifier_push_request_with(
        DelegationVerifierProofPushRequest {
            proof: sample_proof(),
            verifier_targets: vec![verifier_a, verifier_b, out_of_audience],
        },
        DelegationProofInstallIntent::Prewarm,
        root_pid,
        |_principal| true,
    )
    .expect_err("target outside proof audience must fail");

    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(err.message.contains("is not in proof audience"));
}

#[test]
fn normalize_explicit_verifier_push_request_is_idempotent() {
    let root_pid = p(1);
    let verifier_a = p(3);
    let verifier_b = p(4);
    let request = DelegationVerifierProofPushRequest {
        proof: sample_proof(),
        verifier_targets: vec![verifier_a, verifier_a, verifier_b],
    };

    let once = DelegationApi::normalize_explicit_verifier_push_request_with(
        request,
        DelegationProofInstallIntent::Repair,
        root_pid,
        |principal| principal == verifier_a || principal == verifier_b,
    )
    .expect("first normalization should succeed");

    let twice = DelegationApi::normalize_explicit_verifier_push_request_with(
        once.clone(),
        DelegationProofInstallIntent::Repair,
        root_pid,
        |principal| principal == verifier_a || principal == verifier_b,
    )
    .expect("second normalization should succeed");

    assert_eq!(once, twice, "normalization must be idempotent");
}

#[test]
fn normalize_explicit_verifier_push_request_rejects_mixed_targets_without_partial_apply() {
    let root_pid = p(1);
    let verifier_a = p(3);
    let verifier_b = p(4);
    let invalid = p(99);

    let err = DelegationApi::normalize_explicit_verifier_push_request_with(
        DelegationVerifierProofPushRequest {
            proof: sample_proof(),
            verifier_targets: vec![verifier_a, invalid, verifier_b],
        },
        DelegationProofInstallIntent::Repair,
        root_pid,
        |principal| principal == verifier_a || principal == verifier_b,
    )
    .expect_err("mixed valid/invalid targets must fail before fanout");

    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(err.message.contains("is not registered"));
}

#[test]
fn repair_requires_existing_local_proof() {
    let err = DelegationApi::ensure_repair_push_proof_is_locally_available_with(
        &sample_proof(),
        |_proof| Ok(None),
    )
    .expect_err("repair must fail when no local proof exists");

    assert_eq!(err.code, ErrorCode::NotFound);
    assert!(err.message.contains("requires an existing local proof"));
}

#[test]
fn repair_rejects_mismatched_local_proof() {
    let proof = sample_proof();
    let mut stored = sample_proof();
    stored.cert_sig = vec![9, 9, 9];

    let err =
        DelegationApi::ensure_repair_push_proof_is_locally_available_with(&proof, |_candidate| {
            Ok(Some(stored))
        })
        .expect_err("repair must fail when stored proof differs");

    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(err.message.contains("must match the existing local proof"));
}

#[test]
fn repair_accepts_existing_identical_local_proof() {
    let proof = sample_proof();

    DelegationApi::ensure_repair_push_proof_is_locally_available_with(&proof, |_candidate| {
        Ok(Some(proof.clone()))
    })
    .expect("repair should accept identical stored proof");
}

#[test]
fn ensure_target_in_proof_audience_accepts_allowed_verifier() {
    DelegationApi::ensure_target_in_proof_audience(
        &sample_proof(),
        p(3),
        DelegationProofInstallIntent::Repair,
        AudienceBindingFailureStage::PostNormalization,
    )
    .expect("audience-bound target should succeed");
}

#[test]
fn ensure_target_in_proof_audience_rejects_target_outside_audience() {
    let err = DelegationApi::ensure_target_in_proof_audience(
        &sample_proof(),
        p(9),
        DelegationProofInstallIntent::Repair,
        AudienceBindingFailureStage::PostNormalization,
    )
    .expect_err("target outside audience must fail");

    assert_eq!(err.code, ErrorCode::InvalidInput);
    assert!(err.message.contains("is not in proof audience"));
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
