use crate::{
    cdk::types::Principal,
    dto::auth::{AttestationKey, InternalInvocationProofPayloadV1, RoleAttestation},
    ops::auth::{
        AuthExpiryError, AuthOpsError, AuthScopeError, AuthValidationError,
        InternalInvocationProofVerificationInput,
    },
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

// Enforce internal-invocation proof subject, role, method, timing, audience,
// subnet, and epoch bounds before endpoint dispatch.
pub(super) fn verify_internal_invocation_proof_claims(
    payload: &InternalInvocationProofPayloadV1,
    input: InternalInvocationProofVerificationInput<'_>,
) -> Result<(), AuthOpsError> {
    verify_attestation_time_window(payload.issued_at_ns, payload.expires_at_ns, input.now_ns)?;

    if payload.subject != input.caller {
        return Err(AuthScopeError::AttestationSubjectMismatch {
            expected: input.caller,
            found: payload.subject,
        }
        .into());
    }

    if payload.audience != input.self_pid {
        return Err(AuthScopeError::AttestationAudienceMismatch {
            expected: input.self_pid,
            found: payload.audience,
        }
        .into());
    }

    if payload.audience_method != input.target_method {
        return Err(AuthScopeError::InternalInvocationMethodMismatch {
            expected: input.target_method.to_string(),
            found: payload.audience_method.clone(),
        }
        .into());
    }

    if !input
        .accepted_roles
        .iter()
        .any(|role| role == &payload.role)
    {
        return Err(AuthScopeError::InternalInvocationRoleRejected {
            found: payload.role.clone(),
        }
        .into());
    }

    if let Some(attestation_subnet) = payload.subnet_id {
        let verifier_subnet = input
            .verifier_subnet
            .ok_or(AuthValidationError::AttestationSubnetUnavailable)?;
        if attestation_subnet != verifier_subnet {
            return Err(AuthScopeError::AttestationSubnetMismatch {
                expected: verifier_subnet,
                found: attestation_subnet,
            }
            .into());
        }
    }

    if payload.epoch < input.min_accepted_epoch {
        return Err(AuthExpiryError::AttestationEpochRejected {
            epoch: payload.epoch,
            min_accepted_epoch: input.min_accepted_epoch,
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

// Reject attestation keys that are not yet valid or already expired.
pub(super) fn verify_attestation_key_validity(
    key: &AttestationKey,
    now_secs: u64,
) -> Result<(), AuthOpsError> {
    if let Some(valid_from) = key.valid_from
        && now_secs < valid_from
    {
        return Err(AuthExpiryError::AttestationKeyNotYetValid {
            key_id: key.key_id,
            valid_from,
            now_secs,
        }
        .into());
    }

    if let Some(valid_until) = key.valid_until
        && now_secs > valid_until
    {
        return Err(AuthExpiryError::AttestationKeyExpired {
            key_id: key.key_id,
            valid_until,
            now_secs,
        }
        .into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::verify_internal_invocation_proof_claims;
    use crate::{
        cdk::types::Principal,
        dto::auth::{InternalInvocationProofPayloadV1, RoleAttestation},
        ids::CanisterRole,
        ops::auth::{
            AuthExpiryError, AuthOpsError, AuthScopeError, AuthValidationError,
            InternalInvocationProofVerificationInput,
        },
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn payload() -> InternalInvocationProofPayloadV1 {
        InternalInvocationProofPayloadV1 {
            subject: p(1),
            role: CanisterRole::new("project_hub"),
            subnet_id: Some(p(3)),
            audience: p(2),
            audience_method: "system_add_project_to_user".to_string(),
            issued_at_ns: 10,
            expires_at_ns: 20,
            epoch: 4,
        }
    }

    fn input<'a>(
        target_method: &'a str,
        accepted_roles: &'a [CanisterRole],
        min_accepted_epoch: u64,
    ) -> InternalInvocationProofVerificationInput<'a> {
        InternalInvocationProofVerificationInput {
            caller: p(1),
            self_pid: p(2),
            target_method,
            accepted_roles,
            verifier_subnet: Some(p(3)),
            now_ns: 15,
            min_accepted_epoch,
        }
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
    fn internal_invocation_claims_accept_bound_method_role_and_subnet() {
        let accepted = [CanisterRole::new("project_hub")];

        verify_internal_invocation_proof_claims(
            &payload(),
            input("system_add_project_to_user", &accepted, 4),
        )
        .expect("valid internal proof claims");
    }

    #[test]
    fn internal_invocation_claims_reject_method_mismatch() {
        let accepted = [CanisterRole::new("project_hub")];
        let err = verify_internal_invocation_proof_claims(
            &payload(),
            input("other_method", &accepted, 4),
        )
        .expect_err("method mismatch must reject");

        std::assert_matches!(
            err,
            AuthOpsError::Scope(AuthScopeError::InternalInvocationMethodMismatch { .. })
        );
    }

    #[test]
    fn internal_invocation_claims_reject_role_mismatch() {
        let accepted = [CanisterRole::new("admin_hub")];
        let err = verify_internal_invocation_proof_claims(
            &payload(),
            input("system_add_project_to_user", &accepted, 4),
        )
        .expect_err("role mismatch must reject");

        std::assert_matches!(
            err,
            AuthOpsError::Scope(AuthScopeError::InternalInvocationRoleRejected { .. })
        );
    }

    #[test]
    fn internal_invocation_claims_reject_stale_epoch() {
        let accepted = [CanisterRole::new("project_hub")];
        let err = verify_internal_invocation_proof_claims(
            &payload(),
            input("system_add_project_to_user", &accepted, 5),
        )
        .expect_err("stale epoch must reject");

        std::assert_matches!(
            err,
            AuthOpsError::Expiry(AuthExpiryError::AttestationEpochRejected { .. })
        );
    }

    #[test]
    fn internal_invocation_claims_reject_future_issued_at() {
        let accepted = [CanisterRole::new("project_hub")];
        let mut payload = payload();
        payload.issued_at_ns = 16;
        payload.expires_at_ns = 30;

        let err = verify_internal_invocation_proof_claims(
            &payload,
            input("system_add_project_to_user", &accepted, 4),
        )
        .expect_err("future issued_at must reject");

        std::assert_matches!(
            err,
            AuthOpsError::Expiry(AuthExpiryError::AttestationNotYetValid { .. })
        );
    }

    #[test]
    fn internal_invocation_claims_reject_invalid_time_window() {
        let accepted = [CanisterRole::new("project_hub")];
        let mut payload = payload();
        payload.expires_at_ns = payload.issued_at_ns;

        let err = verify_internal_invocation_proof_claims(
            &payload,
            input("system_add_project_to_user", &accepted, 4),
        )
        .expect_err("invalid attestation time window must reject");

        std::assert_matches!(
            err,
            AuthOpsError::Validation(AuthValidationError::AttestationInvalidWindow { .. })
        );
    }

    #[test]
    fn internal_invocation_claims_reject_expiry_boundary() {
        let accepted = [CanisterRole::new("project_hub")];
        let mut payload = payload();
        payload.issued_at_ns = 10;
        payload.expires_at_ns = 15;

        let err = verify_internal_invocation_proof_claims(
            &payload,
            input("system_add_project_to_user", &accepted, 4),
        )
        .expect_err("attestation at expiry boundary must reject");

        std::assert_matches!(
            err,
            AuthOpsError::Expiry(AuthExpiryError::AttestationExpired { .. })
        );
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
