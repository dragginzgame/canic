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
    root_authority: bool,
) -> Result<(), AuthorityRestoreEndpointPolicyError> {
    let snapshot_recovery = fence == AuthorityMutationFence::Snapshot
        && matches!(
            endpoint,
            crate::protocol::protected_state::snapshot_prepare::METHOD
                | crate::protocol::protected_state::snapshot_resume::METHOD
        );
    let root_snapshot_read = fence == AuthorityMutationFence::Snapshot
        && root_authority
        && matches!(
            endpoint,
            crate::protocol::inspection::METHOD | crate::protocol::inspection_history::METHOD
        );
    if fence == AuthorityMutationFence::Open || snapshot_recovery || root_snapshot_read {
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
    #[test]
    fn open_authorities_admit_updates() {
        for root in [false, true] {
            assert_eq!(
                require_update_allowed(
                    AuthorityMutationFence::Open,
                    crate::protocol::batch_control::prepare::METHOD,
                    root
                ),
                Ok(())
            );
        }
    }
    #[test]
    fn snapshot_recovery_reaches_handlers_on_both_authority_roles() {
        for endpoint in [
            crate::protocol::protected_state::snapshot_prepare::METHOD,
            crate::protocol::protected_state::snapshot_resume::METHOD,
        ] {
            for root in [false, true] {
                assert_eq!(
                    require_update_allowed(AuthorityMutationFence::Snapshot, endpoint, root),
                    Ok(())
                );
                assert_eq!(
                    require_update_allowed(AuthorityMutationFence::Release, endpoint, root),
                    Err(AuthorityRestoreEndpointPolicyError::Fenced { endpoint })
                );
            }
        }
    }

    #[test]
    fn snapshots_admit_only_root_inspection_methods() {
        for endpoint in [
            crate::protocol::inspection::METHOD,
            crate::protocol::inspection_history::METHOD,
        ] {
            assert_eq!(
                require_update_allowed(AuthorityMutationFence::Snapshot, endpoint, true),
                Ok(())
            );
            assert_eq!(
                require_update_allowed(AuthorityMutationFence::Snapshot, endpoint, false),
                Err(AuthorityRestoreEndpointPolicyError::Fenced { endpoint })
            );
        }
        let endpoint = crate::protocol::batch_control::prepare::METHOD;
        for root in [false, true] {
            assert_eq!(
                require_update_allowed(AuthorityMutationFence::Snapshot, endpoint, root),
                Err(AuthorityRestoreEndpointPolicyError::Fenced { endpoint })
            );
        }
        assert_eq!(
            require_command_variant_allowed(AuthorityMutationFence::Snapshot, true),
            Ok(())
        );
        assert_eq!(
            require_command_variant_allowed(AuthorityMutationFence::Snapshot, false),
            Err(AuthorityRestoreEndpointPolicyError::FencedCommand)
        );
    }
    #[test]
    fn release_seals_admit_no_inspection_or_ordinary_update() {
        for endpoint in [
            crate::protocol::inspection::METHOD,
            crate::protocol::inspection_history::METHOD,
            crate::protocol::batch_control::prepare::METHOD,
        ] {
            for root in [false, true] {
                assert_eq!(
                    require_update_allowed(AuthorityMutationFence::Release, endpoint, root),
                    Err(AuthorityRestoreEndpointPolicyError::Fenced { endpoint })
                );
            }
        }
        for recovery in [false, true] {
            assert_eq!(
                require_command_variant_allowed(AuthorityMutationFence::Release, recovery),
                Err(AuthorityRestoreEndpointPolicyError::FencedCommand)
            );
        }
    }
}
