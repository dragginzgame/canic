//! Module: ops::auth::delegation::errors
//!
//! Responsibility: map delegated proof helper errors into auth ops errors.
//! Does not own: proof validation, storage, or public error DTO construction.

use super::super::delegated::{
    active_proof::InstallActiveDelegationProofError, delegation_cert::PrepareDelegationCertError,
};
use crate::InternalError;

pub(super) fn map_prepare_delegation_cert_error(err: PrepareDelegationCertError) -> InternalError {
    let code = match err {
        PrepareDelegationCertError::CertTtlZero
        | PrepareDelegationCertError::Audience(_)
        | PrepareDelegationCertError::Canonical(_)
        | PrepareDelegationCertError::CertRules(_) => crate::diagnostics::codes::SECURITY_INVALID,
        PrepareDelegationCertError::CertExpiresAtOverflow => {
            crate::diagnostics::codes::TIME_CAPACITY
        }
    };
    InternalError::public(code)
}

pub(super) fn map_install_active_delegation_proof_error(
    err: InstallActiveDelegationProofError<InternalError>,
) -> InternalError {
    match err {
        InstallActiveDelegationProofError::IssuerMismatch => {
            InternalError::public(crate::diagnostics::codes::SECURITY_CONFLICT)
        }
        InstallActiveDelegationProofError::Canonical(_) => {
            InternalError::public(crate::diagnostics::codes::SECURITY_INVALID)
        }
        InstallActiveDelegationProofError::CertNotYetValid => {
            InternalError::public(crate::diagnostics::codes::SECURITY_INVALID_STATE)
        }
        InstallActiveDelegationProofError::CertExpired => InternalError::auth_proof_expired(),
        InstallActiveDelegationProofError::RootProofInvalid(cause) => cause,
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_proof_install_time_and_identity_failures_keep_public_causes() {
        let cases = [
            (
                InstallActiveDelegationProofError::CertNotYetValid,
                crate::diagnostics::codes::SECURITY_INVALID_STATE.raw_code(),
            ),
            (
                InstallActiveDelegationProofError::CertExpired,
                crate::diagnostics::codes::AUTH_CERT_EXPIRED.raw_code(),
            ),
            (
                InstallActiveDelegationProofError::IssuerMismatch,
                crate::diagnostics::codes::SECURITY_CONFLICT.raw_code(),
            ),
        ];

        for (err, expected) in cases {
            let mapped = map_install_active_delegation_proof_error(err);
            assert_eq!(mapped.public_error().code(), expected);
        }
    }

    #[test]
    fn active_proof_install_preserves_typed_root_proof_cause() {
        let mapped = map_install_active_delegation_proof_error(
            InstallActiveDelegationProofError::RootProofInvalid(
                InternalError::auth_material_stale(),
            ),
        );

        assert_eq!(
            mapped.public_error().code(),
            crate::diagnostics::codes::SECURITY_CONFLICT.raw_code()
        );
    }
}
