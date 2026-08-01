//! Module: ops::auth::verify
//!
//! Responsibility: verify role-attestation claims after proof verification succeeds.
//! Does not own: root proof verification, attestation preparation, or endpoint DTOs.
//! Boundary: private auth-ops semantic verifier for signed role attestations.

#[cfg(test)]
mod tests;

use crate::{
    cdk::types::Principal,
    dto::auth::RoleAttestation,
    ops::auth::{
        AUTH_TIME_SKEW_ALLOWANCE_NS, AuthExpiryError, AuthOpsError, AuthScopeError,
        AuthValidationError,
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

    if issued_at_ns > now_ns.saturating_add(AUTH_TIME_SKEW_ALLOWANCE_NS) {
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
