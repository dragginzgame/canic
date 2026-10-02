//! Module: domain::policy::pure::authority_restore
//!
//! Responsibility: decide which authority updates and command variants may run while sealed.
//! Does not own: stable state reads, authority identity, endpoint dispatch, or mutation.
//! Boundary: workflow supplies validated sealed state and decoded command classification.

use crate::view::authority_restore::AuthorityMutationFence;
use thiserror::Error as ThisError;

/// Failure returned when a sealed authority receives an ordinary update.
#[derive(Debug, Eq, PartialEq, ThisError)]
pub enum AuthorityRestoreEndpointPolicyError {
    #[error("update endpoint {endpoint} is fenced while authority state is sealed")]
    Fenced { endpoint: &'static str },
    #[error("role command is not permitted by the current authority seal")]
    FencedCommand,
}

/// Admit every update while open and only exact recovery updates while sealed.
pub fn require_update_allowed(
    fence: AuthorityMutationFence,
    endpoint: &'static str,
    command_endpoint: &'static str,
) -> Result<(), AuthorityRestoreEndpointPolicyError> {
    if fence == AuthorityMutationFence::Open || endpoint == command_endpoint {
        return Ok(());
    }
    Err(AuthorityRestoreEndpointPolicyError::Fenced { endpoint })
}

/// Admit only decoded snapshot-recovery variants while the authority is sealed.
pub const fn require_command_variant_allowed(
    fence: AuthorityMutationFence,
    recovery_command: bool,
) -> Result<(), AuthorityRestoreEndpointPolicyError> {
    match fence {
        AuthorityMutationFence::Open => Ok(()),
        AuthorityMutationFence::Snapshot if recovery_command => Ok(()),
        AuthorityMutationFence::Snapshot | AuthorityMutationFence::Release => {
            Err(AuthorityRestoreEndpointPolicyError::FencedCommand)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{CANIC_COMMAND, CANIC_COORDINATOR_COMMAND, CANIC_ROOT_COMMAND};

    #[test]
    fn open_authority_admits_updates() {
        assert_eq!(
            require_update_allowed(AuthorityMutationFence::Open, "mutate", CANIC_ROOT_COMMAND),
            Ok(())
        );
    }

    #[test]
    fn sealed_authority_admits_the_dispatcher_then_only_recovery_variants() {
        for command_endpoint in [CANIC_COORDINATOR_COMMAND, CANIC_ROOT_COMMAND] {
            assert_eq!(
                require_update_allowed(
                    AuthorityMutationFence::Snapshot,
                    command_endpoint,
                    command_endpoint
                ),
                Ok(())
            );
            for wrong_endpoint in [CANIC_COMMAND, CANIC_COORDINATOR_COMMAND, CANIC_ROOT_COMMAND]
                .into_iter()
                .filter(|endpoint| *endpoint != command_endpoint)
            {
                assert_eq!(
                    require_update_allowed(
                        AuthorityMutationFence::Snapshot,
                        wrong_endpoint,
                        command_endpoint
                    ),
                    Err(AuthorityRestoreEndpointPolicyError::Fenced {
                        endpoint: wrong_endpoint,
                    })
                );
            }
        }
        assert_eq!(
            require_update_allowed(
                AuthorityMutationFence::Snapshot,
                "mutate",
                CANIC_ROOT_COMMAND
            ),
            Err(AuthorityRestoreEndpointPolicyError::Fenced { endpoint: "mutate" })
        );
        assert_eq!(
            require_command_variant_allowed(AuthorityMutationFence::Snapshot, true),
            Ok(())
        );
        assert_eq!(
            require_command_variant_allowed(AuthorityMutationFence::Snapshot, false),
            Err(AuthorityRestoreEndpointPolicyError::FencedCommand)
        );
        assert_eq!(
            require_command_variant_allowed(AuthorityMutationFence::Open, false),
            Ok(())
        );
    }

    #[test]
    fn release_seal_never_admits_snapshot_recovery_or_ordinary_commands() {
        for command_endpoint in [CANIC_COORDINATOR_COMMAND, CANIC_ROOT_COMMAND] {
            assert_eq!(
                require_update_allowed(
                    AuthorityMutationFence::Release,
                    command_endpoint,
                    command_endpoint
                ),
                Ok(())
            );
            assert!(matches!(
                require_update_allowed(AuthorityMutationFence::Release, "mutate", command_endpoint),
                Err(AuthorityRestoreEndpointPolicyError::Fenced { .. })
            ));
            for recovery in [false, true] {
                assert_eq!(
                    require_command_variant_allowed(AuthorityMutationFence::Release, recovery),
                    Err(AuthorityRestoreEndpointPolicyError::FencedCommand)
                );
            }
        }
    }
}
