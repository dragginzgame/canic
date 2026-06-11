use crate::{
    cdk::types::Principal,
    dto::auth::RoleAttestation,
    ops::auth::{AuthExpiryError, AuthOpsError, AuthScopeError, AuthValidationError},
};

// Enforce role-attestation subject, timing, audience, subnet, and epoch bounds.
pub(super) fn verify_role_attestation_claims(
    payload: &RoleAttestation,
    caller: Principal,
    self_pid: Principal,
    verifier_subnet: Option<Principal>,
    now_ns: u64,
    min_accepted_epoch: u64,
) -> Result<(), AuthOpsError> {
    verify_attestation_time_window(payload.issued_at_ns, payload.expires_at_ns, now_ns)?;

    if payload.subject != caller {
        return Err(AuthScopeError::AttestationSubjectMismatch {
            expected: caller,
            found: payload.subject,
        }
        .into());
    }

    if payload.audience != self_pid {
        return Err(AuthScopeError::AttestationAudienceMismatch {
            expected: self_pid,
            found: payload.audience,
        }
        .into());
    }

    if let Some(attestation_subnet) = payload.subnet_id {
        let verifier_subnet =
            verifier_subnet.ok_or(AuthValidationError::AttestationSubnetUnavailable)?;
        if attestation_subnet != verifier_subnet {
            return Err(AuthScopeError::AttestationSubnetMismatch {
                expected: verifier_subnet,
                found: attestation_subnet,
            }
            .into());
        }
    }

    if payload.epoch < min_accepted_epoch {
        return Err(AuthExpiryError::AttestationEpochRejected {
            epoch: payload.epoch,
            min_accepted_epoch,
        }
        .into());
    }

    Ok(())
}

fn verify_attestation_time_window(
    issued_at_ns: u64,
    expires_at_ns: u64,
    now_ns: u64,
) -> Result<(), AuthOpsError> {
    if expires_at_ns <= issued_at_ns {
        return Err(AuthValidationError::AttestationInvalidWindow {
            issued_at_ns,
            expires_at_ns,
        }
        .into());
    }

    if now_ns < issued_at_ns {
        return Err(AuthExpiryError::AttestationNotYetValid {
            issued_at_ns,
            now_ns,
        }
        .into());
    }

    if now_ns >= expires_at_ns {
        return Err(AuthExpiryError::AttestationExpired {
            expires_at_ns,
            now_ns,
        }
        .into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        cdk::types::Principal,
        dto::auth::RoleAttestation,
        ids::CanisterRole,
        ops::auth::{AuthExpiryError, AuthOpsError, AuthValidationError},
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
    fn role_attestation_claims_reject_future_issued_at() {
        let mut payload = role_attestation();
        payload.issued_at_ns = 16;
        payload.expires_at_ns = 30;

        let err = super::verify_role_attestation_claims(&payload, p(1), p(2), Some(p(3)), 15, 4)
            .expect_err("future issued_at must reject");

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
}
