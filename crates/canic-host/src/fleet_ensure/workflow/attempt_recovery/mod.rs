//! Orchestrate effect-free Host attempt continuation under the existing Fleet operation lock.

use crate::fleet_ensure::{
    model::attempt_recovery::AttemptRecoveryReviewRecord,
    ops::{
        self, EnsurePaths,
        capacity_import::journal::{CapacityImportJournalError, CapacityImportJournalStore},
    },
};
use std::path::Path;

/// Retain finite exhausted-counter continuation for review; no platform call is issued.
pub fn review(
    workspace: &Path,
    environment: &str,
    fleet: &str,
) -> Result<AttemptRecoveryReviewRecord, CapacityImportJournalError> {
    let paths = paths(workspace, environment, fleet)?;
    let _owner = CapacityImportJournalStore::open(&paths)?;
    ops::attempt_recovery::review(&paths, environment, fleet)
}

/// Approve exactly two additional attempts per reviewed resource without resetting consumption.
pub fn apply(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    digest: [u8; 32],
) -> Result<AttemptRecoveryReviewRecord, CapacityImportJournalError> {
    let paths = paths(workspace, environment, fleet)?;
    let _owner = CapacityImportJournalStore::open(&paths)?;
    ops::attempt_recovery::apply(&paths, environment, fleet, digest)
}

fn paths(
    workspace: &Path,
    environment: &str,
    fleet: &str,
) -> Result<EnsurePaths, CapacityImportJournalError> {
    crate::fleet_ensure::policy::validate_path_labels(environment, fleet)
        .map_err(|_| CapacityImportJournalError::RequestInvalid)?;
    Ok(EnsurePaths::under(
        &workspace.canonicalize()?,
        environment,
        fleet,
    ))
}
