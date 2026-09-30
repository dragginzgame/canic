//! Module: restore::persistence::retention
//!
//! Responsibility: bind restore journals to durable backup lifetime references.
//! Does not own: restore command execution or terminal receipt validation.
//! Boundary: references survive owner death and are released only by the terminal runner.

use crate::{
    persistence::{BackupLayout, BackupLayoutGuard, JournalLockError},
    restore::{RestoreApplyJournal, RestoreApplyOperationKind, RestorePersistenceError},
};

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Serialize;
use sha2::{Digest, Sha256};

pub(in crate::restore) fn lock_restore_layout(
    journal: &RestoreApplyJournal,
) -> Result<Option<BackupLayoutGuard>, RestorePersistenceError> {
    let Some(root) = &journal.backup_root else {
        return Ok(None);
    };
    match BackupLayout::new(root.into()).lock_lifetime() {
        Ok(guard) => Ok(Some(guard)),
        // A completed external journal remains replayable after ordinary retention.
        Err(JournalLockError::Io(error))
            if journal.report().complete && error.kind() == io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

pub(in crate::restore) fn retain_restore(
    layout: &BackupLayoutGuard,
    path: &Path,
    journal: &RestoreApplyJournal,
) -> Result<(), RestorePersistenceError> {
    let root_matches = journal
        .backup_root
        .as_ref()
        .is_some_and(|root| Path::new(root) == layout.root());
    if !root_matches {
        return Err(RestorePersistenceError::BackupRootMismatch);
    }
    layout.retain_restore(&journal_identity(path)?, &restore_authority(journal)?)?;
    Ok(())
}

pub(in crate::restore) fn release_restore(
    layout: &BackupLayoutGuard,
    path: &Path,
    journal: &RestoreApplyJournal,
) -> Result<(), RestorePersistenceError> {
    layout.release_restore(&journal_identity(path)?, &restore_authority(journal)?)?;
    Ok(())
}

fn journal_identity(path: &Path) -> Result<PathBuf, RestorePersistenceError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(crate::persistence::PersistenceError::from)?;
    let parent = parent
        .canonicalize()
        .map_err(crate::persistence::PersistenceError::from)?;
    let name = path.file_name().ok_or_else(|| {
        crate::persistence::PersistenceError::Io(io::Error::from(io::ErrorKind::InvalidInput))
    })?;
    Ok(parent.join(name))
}

fn restore_authority(journal: &RestoreApplyJournal) -> Result<String, RestorePersistenceError> {
    let operations = journal
        .operations
        .iter()
        .map(|operation| RestoreOperationAuthority {
            sequence: operation.sequence,
            operation: &operation.operation,
            member_order: operation.member_order,
            source_canister: &operation.source_canister,
            target_canister: &operation.target_canister,
            role: &operation.role,
            snapshot_id: operation.snapshot_id.as_deref(),
            artifact_path: operation.artifact_path.as_deref(),
            artifact_checksum: operation.artifact_checksum.as_ref(),
            expected_module_hash: operation.expected_module_hash.as_deref(),
            verification_kind: operation.verification_kind.as_deref(),
        })
        .collect();
    let authority = RestoreAuthority {
        backup_id: &journal.backup_id,
        operations,
    };
    let bytes =
        serde_json::to_vec(&authority).map_err(crate::persistence::PersistenceError::from)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

///
/// RestoreAuthority
///
/// Immutable backup and operation authority, independent of mutable progress.
///

#[derive(Serialize)]
struct RestoreAuthority<'a> {
    backup_id: &'a str,
    operations: Vec<RestoreOperationAuthority<'a>>,
}

///
/// RestoreOperationAuthority
///
/// Exact inputs that remain unchanged while a restore operation advances.
///

#[derive(Serialize)]
struct RestoreOperationAuthority<'a> {
    sequence: usize,
    operation: &'a RestoreApplyOperationKind,
    member_order: usize,
    source_canister: &'a str,
    target_canister: &'a str,
    role: &'a str,
    snapshot_id: Option<&'a str>,
    artifact_path: Option<&'a str>,
    artifact_checksum: Option<&'a crate::artifacts::ArtifactChecksum>,
    expected_module_hash: Option<&'a str>,
    verification_kind: Option<&'a str>,
}
