//! Module: api::authority_restore
//!
//! Responsibility: expose authority snapshot/restore-fence workflows to generated endpoints.
//! Does not own: authentication, history proof, transition validation, or persistence.
//! Boundary: converts internal workflow failures into the public Canic error contract.

use crate::{
    cdk::types::Principal,
    dto::{
        authority_restore::{AuthorityRestoreFenceStatusResponse, AuthoritySnapshotRequest},
        error::Error,
    },
    workflow::runtime::authority_restore::AuthorityRestoreWorkflow,
};

/// Public facade for controller-owned authority snapshot fencing.
pub struct AuthorityRestoreApi;

impl AuthorityRestoreApi {
    #[doc(hidden)]
    pub fn initialize(authority_canister: Principal) -> Result<(), Error> {
        AuthorityRestoreWorkflow::initialize(authority_canister).map_err(Into::into)
    }

    /// Return the authority's durable snapshot-fence state.
    pub fn status() -> Result<AuthorityRestoreFenceStatusResponse, Error> {
        AuthorityRestoreWorkflow::status().map_err(Into::into)
    }

    #[doc(hidden)]
    pub fn require_command_variant_allowed(recovery_command: bool) -> Result<(), Error> {
        AuthorityRestoreWorkflow::require_command_variant_allowed(recovery_command)
            .map_err(Into::into)
    }

    /// Seal ordinary mutation before an external authority snapshot is captured.
    pub async fn prepare_snapshot(
        request: AuthoritySnapshotRequest,
    ) -> Result<AuthorityRestoreFenceStatusResponse, Error> {
        AuthorityRestoreWorkflow::prepare_snapshot(request)
            .await
            .map_err(Into::into)
    }

    /// Resume the live authority only when its independent history still matches the seal.
    pub async fn resume_snapshot(
        request: AuthoritySnapshotRequest,
    ) -> Result<AuthorityRestoreFenceStatusResponse, Error> {
        AuthorityRestoreWorkflow::resume_snapshot(request)
            .await
            .map_err(Into::into)
    }
}
