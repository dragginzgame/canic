//! Module: restore::persistence
//!
//! Responsibility: validate and durably persist restore recovery documents.
//! Does not own: restore planning, journal transitions, or generic CLI output.
//! Boundary: exposes typed plan/journal writes backed by backup-owned durable IO.

mod retention;

use super::{RestoreApplyJournal, RestoreApplyJournalError, RestorePlan, RestorePlanError};
use crate::persistence::{
    BackupLayout, BackupLayoutGuard, JournalLock, JournalLockError, PersistenceError,
    create_json_durable, read_json, write_json_durable,
};

use std::{io, path::Path};

use thiserror::Error as ThisError;

pub(in crate::restore) use retention::{lock_restore_layout, release_restore, retain_restore};

///
/// RestorePersistenceError
///
/// Typed validation or durable-write failure for restore recovery documents.
/// Owned by restore persistence and returned to CLI and runner callers.
///

#[derive(Debug, ThisError)]
pub enum RestorePersistenceError {
    #[error("restore journal backup root does not match the locked layout")]
    BackupRootMismatch,

    #[error(transparent)]
    Lock(#[from] JournalLockError),

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
    layout: &BackupLayoutGuard,
    path: &Path,
    journal: &RestoreApplyJournal,
) -> Result<(), RestorePersistenceError> {
    journal.validate()?;
    let _lock = lock_journal_for_publication(path)?;
    match read_json::<RestoreApplyJournal>(path) {
        Ok(existing) => {
            existing.validate()?;
            if existing == *journal {
                retain_restore(layout, path, journal)?;
                Ok(())
            } else {
                Err(RestorePersistenceError::ApplyJournalConflict {
                    path: path.display().to_string(),
                })
            }
        }
        Err(PersistenceError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            retain_restore(layout, path, journal)?;
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
    // Acquire the existing source before creating a journal parent. Otherwise a
    // default journal path could recreate a backup that prune has already removed.
    let layout = journal
        .backup_root
        .as_ref()
        .map(|root| BackupLayout::new(root.into()).lock_lifetime())
        .transpose()?;
    let _lock = lock_journal_for_publication(path)?;
    if let Some(layout) = &layout {
        retain_restore(layout, path, journal)?;
    }
    write_json_durable(path, journal)?;
    Ok(())
}

fn lock_journal_for_publication(path: &Path) -> Result<JournalLock, RestorePersistenceError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(PersistenceError::from)?;
    Ok(JournalLock::acquire(path)?)
}
