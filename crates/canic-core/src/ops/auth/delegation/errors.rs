//! Module: ops::auth::delegation::errors
//!
//! Responsibility: map delegated proof helper errors into auth ops errors.
//! Does not own: proof validation, storage, or public error DTO construction.

use super::super::delegated::{
    active_proof::InstallActiveDelegationProofError, delegation_cert::PrepareDelegationCertError,
};
use crate::{InternalError, domain::policy::auth::AuthPolicyError, ops::auth::AuthValidationError};

pub(super) fn map_prepare_delegation_cert_error(err: PrepareDelegationCertError) -> InternalError {
    AuthValidationError::Auth(err.to_string()).into()
}

pub(super) fn map_install_active_delegation_proof_error(
    err: InstallActiveDelegationProofError,
) -> InternalError {
    AuthValidationError::Auth(err.to_string()).into()
}

pub(super) fn map_root_provisioning_policy_error(err: AuthPolicyError) -> InternalError {
    InternalError::forbidden(err.to_string())
}
