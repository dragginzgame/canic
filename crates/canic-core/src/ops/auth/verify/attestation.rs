use crate::{
    cdk::types::Principal,
    dto::auth::{AttestationKey, RoleAttestation},
    ops::auth::{
        DelegatedTokenOpsError, DelegationExpiryError, DelegationScopeError,
        DelegationValidationError,
    },
};

// Enforce role-attestation subject, timing, audience, subnet, and epoch bounds.
pub(super) fn verify_role_attestation_claims(
    payload: &RoleAttestation,
    caller: Principal,
    self_pid: Principal,
    verifier_subnet: Option<Principal>,
    now_secs: u64,
    min_accepted_epoch: u64,
) -> Result<(), DelegatedTokenOpsError> {
    if payload.subject != caller {
        return Err(DelegationScopeError::AttestationSubjectMismatch {
            expected: caller,
            found: payload.subject,
        }
        .into());
    }

    if now_secs > payload.expires_at {
        return Err(DelegationExpiryError::AttestationExpired {
            expires_at: payload.expires_at,
            now_secs,
        }
        .into());
    }

    if let Some(audience) = payload.audience
        && audience != self_pid
    {
        return Err(DelegationScopeError::AttestationAudienceMismatch {
            expected: self_pid,
            found: audience,
        }
        .into());
    }

    if let Some(attestation_subnet) = payload.subnet_id {
        let verifier_subnet =
            verifier_subnet.ok_or(DelegationValidationError::AttestationSubnetUnavailable)?;
        if attestation_subnet != verifier_subnet {
            return Err(DelegationScopeError::AttestationSubnetMismatch {
                expected: verifier_subnet,
                found: attestation_subnet,
            }
            .into());
        }
    }

    if payload.epoch < min_accepted_epoch {
        return Err(DelegationExpiryError::AttestationEpochRejected {
            epoch: payload.epoch,
            min_accepted_epoch,
        }
        .into());
    }

    Ok(())
}

// Reject attestation keys that are not yet valid or already expired.
pub(super) fn verify_attestation_key_validity(
    key: &AttestationKey,
    now_secs: u64,
) -> Result<(), DelegatedTokenOpsError> {
    if let Some(valid_from) = key.valid_from
        && now_secs < valid_from
    {
        return Err(DelegationExpiryError::AttestationKeyNotYetValid {
            key_id: key.key_id,
            valid_from,
            now_secs,
        }
        .into());
    }

    if let Some(valid_until) = key.valid_until
        && now_secs > valid_until
    {
        return Err(DelegationExpiryError::AttestationKeyExpired {
            key_id: key.key_id,
            valid_until,
            now_secs,
        }
        .into());
    }

    Ok(())
}
