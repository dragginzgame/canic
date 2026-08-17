//! Module: domain::policy::pure::authority_restore
//!
//! Responsibility: decide which authority updates and command variants may run while sealed.
//! Does not own: stable state reads, authority identity, endpoint dispatch, or mutation.
//! Boundary: workflow supplies validated sealed state and decoded command classification.

use crate::protocol::CANIC_COMMAND;
use thiserror::Error as ThisError;

/// Failure returned when a sealed authority receives an ordinary update.
#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum AuthorityRestoreEndpointPolicyError {
    #[error("update endpoint {endpoint} is fenced while authority snapshot state is sealed")]
    Fenced { endpoint: &'static str },
    #[error("ordinary role command is fenced while authority snapshot state is sealed")]
    FencedCommand,
}

/// Admit every update while open and only exact recovery updates while sealed.
pub fn require_update_allowed(
    is_sealed: bool,
    endpoint: &'static str,
) -> Result<(), AuthorityRestoreEndpointPolicyError> {
    if !is_sealed || endpoint == CANIC_COMMAND {
        return Ok(());
    }
    Err(AuthorityRestoreEndpointPolicyError::Fenced { endpoint })
}

/// Admit only decoded snapshot-recovery variants while the authority is sealed.
pub const fn require_command_variant_allowed(
    is_sealed: bool,
    recovery_command: bool,
) -> Result<(), AuthorityRestoreEndpointPolicyError> {
    if !is_sealed || recovery_command {
        return Ok(());
    }
    Err(AuthorityRestoreEndpointPolicyError::FencedCommand)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_authority_admits_updates() {
        assert_eq!(require_update_allowed(false, "mutate"), Ok(()));
    }

    #[test]
    fn sealed_authority_admits_the_dispatcher_then_only_recovery_variants() {
        assert_eq!(require_update_allowed(true, CANIC_COMMAND), Ok(()));
        assert_eq!(
            require_update_allowed(true, "mutate"),
            Err(AuthorityRestoreEndpointPolicyError::Fenced { endpoint: "mutate" })
        );
        assert_eq!(require_command_variant_allowed(true, true), Ok(()));
        assert_eq!(
            require_command_variant_allowed(true, false),
            Err(AuthorityRestoreEndpointPolicyError::FencedCommand)
        );
        assert_eq!(require_command_variant_allowed(false, false), Ok(()));
    }
}
