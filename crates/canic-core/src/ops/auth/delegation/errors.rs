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
    err: InstallActiveDelegationProofError,
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
        // Preserve installation's existing retry classification; token presentation
        // maps inconsistent submitted metadata to invalid input at its own boundary.
        #[cfg(any(feature = "auth-chain-key-ecdsa", test))]
        InstallActiveDelegationProofError::RootProofInvalid(
            ic_auth::token::TokenVerificationError::RootSignatureBindingMismatch { .. },
        ) => InternalError::auth_material_stale(),
        #[cfg(any(feature = "auth-chain-key-ecdsa", test))]
        InstallActiveDelegationProofError::RootProofInvalid(cause) => {
            crate::ops::auth::delegated::root_proof::map_verification_error(cause)
        }
        #[cfg(any(feature = "auth-chain-key-ecdsa", test))]
        InstallActiveDelegationProofError::RootPolicyRejected => InternalError::invalid_input(),
        #[cfg(not(any(feature = "auth-chain-key-ecdsa", test)))]
        InstallActiveDelegationProofError::VerificationUnavailable => {
            InternalError::public(crate::diagnostics::codes::SECURITY_UNAVAILABLE)
        }
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
        use ic_auth::token::TokenVerificationError;
        for cause in [
            TokenVerificationError::StaleAuthority {
                field: "proof_epoch",
            },
            TokenVerificationError::RootSignatureBindingMismatch { field: "key_id" },
            TokenVerificationError::RootKeyPolicyBindingMismatch { field: "key_id" },
        ] {
            let mapped = map_install_active_delegation_proof_error(
                InstallActiveDelegationProofError::RootProofInvalid(cause),
            );
            assert_eq!(
                mapped.public_error().code(),
                crate::diagnostics::codes::SECURITY_CONFLICT.raw_code()
            );
        }
    }
}
