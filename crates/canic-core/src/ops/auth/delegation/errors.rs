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
        #[cfg(any(feature = "auth-chain-key-ecdsa", test))]
        InstallActiveDelegationProofError::RootProofInvalid(cause) => match cause {
            ic_auth::token::TokenVerificationError::Expired {
                target: "root_key_policy",
            }
            | ic_auth::token::TokenVerificationError::StaleAuthority { .. }
            | ic_auth::token::TokenVerificationError::BindingMismatch {
                field:
                    "key_id" | "root_public_key" | "key_version" | "derivation_path_hash" | "algorithm",
            } => InternalError::auth_material_stale(),
            ic_auth::token::TokenVerificationError::Expired { .. } => {
                InternalError::auth_proof_expired()
            }
            ic_auth::token::TokenVerificationError::NotYetValid { .. } => {
                InternalError::auth_proof_pending()
            }
            _ => InternalError::invalid_input(),
        },
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
        let mapped = map_install_active_delegation_proof_error(
            InstallActiveDelegationProofError::RootProofInvalid(
                ic_auth::token::TokenVerificationError::StaleAuthority {
                    field: "proof_epoch",
                },
            ),
        );

        assert_eq!(
            mapped.public_error().code(),
            crate::diagnostics::codes::SECURITY_CONFLICT.raw_code()
        );
    }
}
