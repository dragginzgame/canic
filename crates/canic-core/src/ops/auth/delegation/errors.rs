//! Module: ops::auth::delegation::errors
//!
//! Responsibility: map delegated proof helper errors into auth ops errors.
//! Does not own: proof validation, storage, or public error DTO construction.

use super::super::delegated::{
    active_proof::InstallActiveDelegationProofError, delegation_cert::PrepareDelegationCertError,
};
use crate::{InternalError, domain::policy::pure::auth::AuthPolicyError};

pub(super) fn map_prepare_delegation_cert_error(err: PrepareDelegationCertError) -> InternalError {
    InternalError::invalid_input(err.to_string())
}

pub(super) fn map_install_active_delegation_proof_error(
    err: InstallActiveDelegationProofError<InternalError>,
) -> InternalError {
    match err {
        err @ (InstallActiveDelegationProofError::IssuerMismatch
        | InstallActiveDelegationProofError::Canonical(_)) => {
            InternalError::invalid_input(err.to_string())
        }
        err @ InstallActiveDelegationProofError::CertNotYetValid => {
            InternalError::auth_proof_pending(err.to_string())
        }
        err @ InstallActiveDelegationProofError::CertExpired => {
            InternalError::auth_proof_expired(err.to_string())
        }
        InstallActiveDelegationProofError::RootProofInvalid(cause) => cause,
    }
}

pub(super) fn map_root_provisioning_policy_error(err: AuthPolicyError) -> InternalError {
    InternalError::forbidden(err.to_string())
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::error::ErrorCode;

    #[test]
    fn active_proof_install_time_and_identity_failures_keep_public_causes() {
        let cases = [
            (
                InstallActiveDelegationProofError::CertNotYetValid,
                ErrorCode::AuthProofPending,
            ),
            (
                InstallActiveDelegationProofError::CertExpired,
                ErrorCode::AuthProofExpired,
            ),
            (
                InstallActiveDelegationProofError::IssuerMismatch,
                ErrorCode::InvalidInput,
            ),
        ];

        for (err, expected) in cases {
            let mapped = map_install_active_delegation_proof_error(err);
            assert_eq!(mapped.public_error().map(|err| err.code), Some(expected));
        }
    }

    #[test]
    fn active_proof_install_preserves_typed_root_proof_cause() {
        let mapped = map_install_active_delegation_proof_error(
            InstallActiveDelegationProofError::RootProofInvalid(
                InternalError::auth_material_stale("root policy changed"),
            ),
        );

        assert_eq!(
            mapped.public_error().map(|err| err.code),
            Some(ErrorCode::AuthMaterialStale)
        );
    }
}
