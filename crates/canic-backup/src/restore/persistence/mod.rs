//! Module: restore::persistence
//!
//! Responsibility: validate and durably persist restore recovery documents.
//! Does not own: restore planning, journal transitions, or generic CLI output.
//! Boundary: exposes typed plan/journal writes backed by backup-owned durable IO.

use super::{RestoreApplyJournal, RestoreApplyJournalError, RestorePlan, RestorePlanError};
use crate::persistence::{PersistenceError, create_json_durable, read_json, write_json_durable};

use std::{io, path::Path};

use thiserror::Error as ThisError;

///
/// RestorePersistenceError
///
/// Typed validation or durable-write failure for restore recovery documents.
/// Owned by restore persistence and returned to CLI and runner callers.
///

#[derive(Debug, ThisError)]
pub enum RestorePersistenceError {
    #[error("restore apply journal conflicts with the existing recovery document: {path}")]
    ApplyJournalConflict { path: String },

    #[error(transparent)]
    InvalidPlan(#[from] RestorePlanError),

    #[error(transparent)]
    InvalidJournal(#[from] RestoreApplyJournalError),

    #[error(transparent)]
    Persistence(#[from] PersistenceError),

    #[error("restore plan conflicts with the existing recovery document: {path}")]
    PlanConflict { path: String },
}

/// Create a restore plan, or adopt an exact existing plan without replacing it.
pub fn create_or_adopt_restore_plan(
    path: &Path,
    plan: &RestorePlan,
) -> Result<(), RestorePersistenceError> {
    plan.validate()?;
    match read_json::<RestorePlan>(path) {
        Ok(existing) => {
            existing.validate()?;
            if existing == *plan {
                Ok(())
            } else {
                Err(RestorePersistenceError::PlanConflict {
                    path: path.display().to_string(),
                })
            }
        }
        Err(PersistenceError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            create_json_durable(path, plan).map_err(RestorePersistenceError::from)
        }
        Err(error) => Err(error.into()),
    }
}

/// Create a pristine restore journal, or adopt the exact existing journal.
pub fn create_or_adopt_restore_apply_journal(
    path: &Path,
    journal: &RestoreApplyJournal,
) -> Result<(), RestorePersistenceError> {
    journal.validate()?;
    match read_json::<RestoreApplyJournal>(path) {
        Ok(existing) => {
            existing.validate()?;
            if existing == *journal {
                Ok(())
            } else {
                Err(RestorePersistenceError::ApplyJournalConflict {
                    path: path.display().to_string(),
                })
            }
        }
        Err(PersistenceError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            create_json_durable(path, journal).map_err(RestorePersistenceError::from)
        }
        Err(error) => Err(error.into()),
    }
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
