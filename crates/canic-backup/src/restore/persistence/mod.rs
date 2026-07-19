//! Module: restore::persistence
//!
//! Responsibility: validate and durably persist restore recovery documents.
//! Does not own: restore planning, journal transitions, or generic CLI output.
//! Boundary: exposes typed plan/journal writes backed by backup-owned durable IO.

use super::{RestoreApplyJournal, RestoreApplyJournalError, RestorePlan, RestorePlanError};
use crate::persistence::{PersistenceError, write_json_durable};

use std::path::Path;

use thiserror::Error as ThisError;

///
/// RestorePersistenceError
///
/// Typed validation or durable-write failure for restore recovery documents.
/// Owned by restore persistence and returned to CLI and runner callers.
///

#[derive(Debug, ThisError)]
pub enum RestorePersistenceError {
    #[error(transparent)]
    InvalidPlan(#[from] RestorePlanError),

    #[error(transparent)]
    InvalidJournal(#[from] RestoreApplyJournalError),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),
}

/// Durably replace one serialized restore plan.
pub fn write_restore_plan(path: &Path, plan: &RestorePlan) -> Result<(), RestorePersistenceError> {
    plan.validate()?;
    write_json_durable(path, plan)?;
    Ok(())
}

/// Validate and durably replace one restore apply journal.
pub fn write_restore_apply_journal(
    path: &Path,
    journal: &RestoreApplyJournal,
) -> Result<(), RestorePersistenceError> {
    journal.validate()?;
    write_json_durable(path, journal)?;
    Ok(())
}
