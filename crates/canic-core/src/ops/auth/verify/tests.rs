use crate::{
    cdk::types::Principal,
    dto::auth::RoleAttestation,
    ids::CanisterRole,
    ops::auth::{
        AUTH_TIME_SKEW_ALLOWANCE_NS, AuthExpiryError, AuthOpsError, AuthScopeError,
        AuthValidationError,
    },
};

fn p(id: u8) -> Principal {
    Principal::from_slice(&[id; 29])
}

fn role_attestation() -> RoleAttestation {
    RoleAttestation {
        subject: p(1),
        role: CanisterRole::new("project_hub"),
        subnet_id: Some(p(3)),
        audience: p(2),
        issued_at_ns: 10,
        expires_at_ns: 20,
        epoch: 4,
    }
}

#[test]
fn role_attestation_claims_accept_future_issued_at_within_skew() {
    let mut payload = role_attestation();
    payload.issued_at_ns = 15 + 30_000_000_000;
    payload.expires_at_ns = payload.issued_at_ns + 10;

    super::verify_role_attestation_claims(&payload, p(1), p(2), Some(p(3)), 15, 4)
        .expect("future issued_at within skew allowance should verify");
}

#[test]
fn role_attestation_claims_reject_subject_audience_subnet_and_epoch_drift() {
    let payload = role_attestation();

    let subject = super::verify_role_attestation_claims(&payload, p(9), p(2), Some(p(3)), 15, 4)
        .expect_err("wrong caller must reject");
    std::assert_matches!(
        subject,
        AuthOpsError::Scope(AuthScopeError::AttestationSubjectMismatch { .. })
    );

    let audience = super::verify_role_attestation_claims(&payload, p(1), p(9), Some(p(3)), 15, 4)
        .expect_err("wrong audience must reject");
    std::assert_matches!(
        audience,
        AuthOpsError::Scope(AuthScopeError::AttestationAudienceMismatch { .. })
    );

    let subnet = super::verify_role_attestation_claims(&payload, p(1), p(2), Some(p(9)), 15, 4)
        .expect_err("wrong subnet must reject");
    std::assert_matches!(
        subnet,
        AuthOpsError::Scope(AuthScopeError::AttestationSubnetMismatch { .. })
    );

    let epoch = super::verify_role_attestation_claims(&payload, p(1), p(2), Some(p(3)), 15, 5)
        .expect_err("stale epoch must reject");
    std::assert_matches!(
        epoch,
        AuthOpsError::Expiry(AuthExpiryError::AttestationEpochRejected { .. })
    );
}

#[test]
fn role_attestation_claims_reject_future_issued_at_beyond_skew() {
    let mut payload = role_attestation();
    payload.issued_at_ns = 15 + AUTH_TIME_SKEW_ALLOWANCE_NS + 1;
    payload.expires_at_ns = payload.issued_at_ns + 10;

    let err = super::verify_role_attestation_claims(&payload, p(1), p(2), Some(p(3)), 15, 4)
        .expect_err("future issued_at beyond skew allowance must reject");

    std::assert_matches!(
        err,
        AuthOpsError::Expiry(AuthExpiryError::AttestationNotYetValid { .. })
    );
}

#[test]
fn role_attestation_claims_reject_invalid_time_window() {
    let mut payload = role_attestation();
    payload.expires_at_ns = payload.issued_at_ns;

    let err = super::verify_role_attestation_claims(&payload, p(1), p(2), Some(p(3)), 15, 4)
        .expect_err("invalid attestation time window must reject");

    std::assert_matches!(
        err,
        AuthOpsError::Validation(AuthValidationError::AttestationInvalidWindow { .. })
    );
}

#[test]
fn role_attestation_claims_reject_expiry_boundary() {
    let mut payload = role_attestation();
    payload.issued_at_ns = 10;
    payload.expires_at_ns = 15;

    let err = super::verify_role_attestation_claims(&payload, p(1), p(2), Some(p(3)), 15, 4)
        .expect_err("attestation at expiry boundary must reject");

    std::assert_matches!(
        err,
        AuthOpsError::Expiry(AuthExpiryError::AttestationExpired { .. })
    );
}
