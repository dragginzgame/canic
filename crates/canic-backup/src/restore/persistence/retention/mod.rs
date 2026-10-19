//! Module: restore::persistence::retention
//!
//! Responsibility: bind restore journals to durable backup lifetime references.
//! Does not own: restore command execution or terminal receipt validation.
//! Boundary: references survive owner death and are released only by the terminal runner.

use crate::{
    persistence::{BackupLayout, BackupLayoutGuard, JournalLockError},
    restore::{RestoreApplyJournal, RestoreApplyOperationKind, RestorePersistenceError},
};
use ic_backup::model::artifacts::ArtifactChecksumRecord;

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use ic_host_artifacts::artifact::Sha256Digest;
use serde::Serialize;

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
    layout
        .retain_restore(&journal_identity(path)?, &restore_authority(journal)?)
        .map_err(crate::persistence::PersistenceError::from)?;
    Ok(())
}

pub(in crate::restore) fn release_restore(
    layout: &BackupLayoutGuard,
    journal: &crate::persistence::JournalLock,
) -> Result<(), RestorePersistenceError> {
    layout.release_restore(
        journal,
        &CanicTerminalEvidence {
            root: Some(layout.root()),
        },
    )?;
    Ok(())
}

/// Refusal of original restore terminal evidence; no refusal authorizes replay.
#[derive(Debug, thiserror::Error)]
pub enum RestoreTerminalEvidenceError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Journal(#[from] crate::restore::RestoreApplyJournalError),
    #[error(transparent)]
    Receipt(Box<crate::restore::RestoreRunnerError>),
    #[error(transparent)]
    Authority(Box<RestorePersistenceError>),
    #[error(transparent)]
    Checksum(#[from] ic_backup::model::artifacts::ChecksumError),
    #[error("restore source differs from its held backup layout")]
    BackupRootMismatch,
    #[error("original restore is not terminal")]
    Incomplete,
    #[error("original restore command custody is incomplete or conflicting")]
    CommandSet,
}

struct CanicTerminalEvidence<'a> {
    root: Option<&'a Path>,
}

impl ic_backup::ops::persistence::RestoreTerminalEvidenceProvider for CanicTerminalEvidence<'_> {
    type Error = RestoreTerminalEvidenceError;

    fn qualify_terminal(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> Result<ic_backup::view::restore_reference_release::TerminalRestoreEvidence, Self::Error>
    {
        let journal: RestoreApplyJournal = serde_json::from_slice(bytes)?;
        journal.validate()?;
        if self.root.is_some_and(|expected| {
            journal
                .backup_root
                .as_deref()
                .is_none_or(|root| Path::new(root) != expected)
        }) {
            return Err(RestoreTerminalEvidenceError::BackupRootMismatch);
        }
        crate::restore::runner::io::validate_terminal_operation_receipts(&journal)
            .map_err(|error| RestoreTerminalEvidenceError::Receipt(Box::new(error)))?;
        if !journal.report().complete {
            return Err(RestoreTerminalEvidenceError::Incomplete);
        }
        let expected = journal
            .operations
            .iter()
            .filter(|operation| {
                matches!(
                    operation.operation,
                    RestoreApplyOperationKind::UploadSnapshot
                        | RestoreApplyOperationKind::StopCanister
                        | RestoreApplyOperationKind::LoadSnapshot
                        | RestoreApplyOperationKind::StartCanister
                )
            })
            .map(|operation| operation.sequence as u64)
            .collect::<std::collections::BTreeSet<_>>();
        let mut observed = std::collections::BTreeSet::new();
        for command in &journal.command_custody {
            if command.journal() != path
                || !expected.contains(&command.operation_sequence())
                || !observed.insert(command.operation_sequence())
            {
                return Err(RestoreTerminalEvidenceError::CommandSet);
            }
        }
        if observed != expected {
            return Err(RestoreTerminalEvidenceError::CommandSet);
        }
        let authority = ic_backup::model::artifacts::ArtifactChecksumRecord::from_hash(
            &restore_authority(&journal)
                .map_err(|error| RestoreTerminalEvidenceError::Authority(Box::new(error)))?,
        )?;
        Ok(
            ic_backup::view::restore_reference_release::TerminalRestoreEvidence::new(
                authority,
                journal.command_custody,
            ),
        )
    }
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
    Ok(Sha256Digest::compute(&bytes).to_string())
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
    artifact_checksum: Option<&'a ArtifactChecksumRecord>,
    expected_module_hash: Option<&'a str>,
    verification_kind: Option<&'a str>,
}

// Completed external journals remain replayable after source retention, with exact custody.
pub(in crate::restore) fn terminal_evidence_without_layout(
    journal: &crate::persistence::JournalLock,
) -> Result<
    ic_backup::view::restore_reference_release::TerminalRestoreEvidence,
    RestorePersistenceError,
> {
    use ic_backup::ops::persistence::RestoreTerminalEvidenceProvider as _;
    let bytes = ic_host_fs::read::read_file_no_follow(
        journal.journal(),
        ic_backup::ops::persistence::MAX_RESTORE_TERMINAL_BYTES,
    )
    .map_err(std::io::Error::other)
    .map_err(crate::persistence::PersistenceError::from)?;
    CanicTerminalEvidence { root: None }
        .qualify_terminal(journal.journal(), &bytes)
        .map_err(RestorePersistenceError::TerminalEvidence)
}
