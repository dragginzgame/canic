//! Inspect retained operation completion before admitting current Fleet work.
//!
//! Completed payloads remain opaque history. Unfinished paid imports and local
//! activation handoffs retain their exact recovery owner.

use crate::fleet_ensure::{
    ops::{EnsurePaths, EnsureStateError, operation_selection},
    policy::{EnsurePolicyError, validate_path_labels},
};
use std::path::Path;
use thiserror::Error;

/// Retained operation metadata or policy could not establish a safe owner.
#[derive(Debug, Error)]
pub enum RetainedContractError {
    #[error("retained operation admission failed: {0}")]
    State(#[from] EnsureStateError),
    #[error(transparent)]
    Policy(#[from] EnsurePolicyError),
}

/// Require unresolved paid effects to retain their current recovery owner.
pub fn check(
    workspace: &Path,
    environment: &str,
    fleet: &str,
) -> Result<(), RetainedContractError> {
    validate_path_labels(environment, fleet)?;
    let paths = EnsurePaths::under(workspace, environment, fleet);
    crate::fleet_ensure::ops::clean_reinstall::cancellation::require_no_pending(&paths)?;
    crate::fleet_ensure::ops::capacity_import::journal::require_no_approved_import(&paths)?;
    if operation_selection::completed(&paths, environment, fleet)?.is_some() {
        operation_selection::retirement::require_terminal_side_effects(&paths)?;
    }
    Ok(())
}
