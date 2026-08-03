//! Module: domain::policy::pure::authority_restore
//!
//! Responsibility: decide which authority updates may run while snapshot-sealed.
//! Does not own: stable state reads, authority identity, endpoint dispatch, or mutation.
//! Boundary: workflow supplies validated sealed state and one exact endpoint name.

use crate::protocol::{CANIC_AUTHORITY_SNAPSHOT_PREPARE, CANIC_AUTHORITY_SNAPSHOT_RESUME};
use thiserror::Error as ThisError;

/// Failure returned when a sealed authority receives an ordinary update.
#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum AuthorityRestoreEndpointPolicyError {
    #[error("update endpoint {endpoint} is fenced while authority snapshot state is sealed")]
    Fenced { endpoint: &'static str },
}

/// Admit every update while open and only exact recovery updates while sealed.
pub fn require_update_allowed(
    is_sealed: bool,
    endpoint: &'static str,
) -> Result<(), AuthorityRestoreEndpointPolicyError> {
    if !is_sealed || is_recovery_endpoint(endpoint) {
        return Ok(());
    }
    Err(AuthorityRestoreEndpointPolicyError::Fenced { endpoint })
}

fn is_recovery_endpoint(endpoint: &str) -> bool {
    matches!(
        endpoint,
        CANIC_AUTHORITY_SNAPSHOT_PREPARE | CANIC_AUTHORITY_SNAPSHOT_RESUME
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_authority_admits_updates() {
        assert_eq!(require_update_allowed(false, "mutate"), Ok(()));
    }

    #[test]
    fn sealed_authority_admits_only_recovery_updates() {
        for endpoint in [
            CANIC_AUTHORITY_SNAPSHOT_PREPARE,
            CANIC_AUTHORITY_SNAPSHOT_RESUME,
        ] {
            assert_eq!(require_update_allowed(true, endpoint), Ok(()));
        }
        assert_eq!(
            require_update_allowed(true, "mutate"),
            Err(AuthorityRestoreEndpointPolicyError::Fenced { endpoint: "mutate" })
        );
    }
}
