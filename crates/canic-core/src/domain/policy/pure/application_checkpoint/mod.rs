//! Module: domain::policy::pure::application_checkpoint
//!
//! Responsibility: admit the exact installed Root and receiver checkpoint audience.
//! Does not own: storage, application freeze, sequence issuance or endpoint evaluation.
//! Boundary: the current authority is supplied independently of the request.

#[cfg(test)]
mod tests;

use crate::model::caller_authority::CallerAdmissionError;
use canic_contracts::ids::{ApplicationCheckpointBinding, CallerReceiverAuthority};
use ic_platform::Principal;

/// Root recovery authority is independent from ordinary application admission.
pub fn require_root(
    requested: &ApplicationCheckpointBinding,
    current: &CallerReceiverAuthority,
    caller: Principal,
) -> Result<(), CallerAdmissionError> {
    if caller != current.issuer.root {
        return Err(CallerAdmissionError::PermissionDenied);
    }
    if requested.authority != *current {
        return Err(CallerAdmissionError::AuthorityConflict);
    }
    Ok(())
}
