//! Module: restore::runner::artifact
//!
//! Responsibility: privately stage the exact checksum-bound restore artifact bytes.
//! Does not own: restore planning, journal transitions, or command execution.
//! Boundary: execution receives only a no-follow descriptor-copied artifact path.

use crate::{artifacts::ArtifactChecksum, persistence::resolve_backup_artifact_path};

use std::{
    fs::{self, DirBuilder},
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

use super::{
    RestoreApplyJournal, RestoreApplyJournalOperation, RestoreApplyOperationKind,
    RestoreRunnerConfig, RestoreRunnerError,
};

pub(super) struct StagedRestoreArtifact {
    artifact_path: PathBuf,
    operation_root: PathBuf,
    stage_root: PathBuf,
}

impl StagedRestoreArtifact {
    pub(super) fn artifact_path(&self) -> &Path {
        self.artifact_path.as_path()
    }
}

impl Drop for StagedRestoreArtifact {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.operation_root);
        let _ = fs::remove_dir(&self.stage_root);
    }
}

pub(super) fn stage_upload_artifact(
    config: &RestoreRunnerConfig,
    journal: &RestoreApplyJournal,
    operation: &RestoreApplyJournalOperation,
) -> Result<Option<StagedRestoreArtifact>, RestoreRunnerError> {
    if operation.operation != RestoreApplyOperationKind::UploadSnapshot {
        return Ok(None);
    }

    let backup_root = required_path(
        operation.sequence,
        "journal.backup_root",
        journal.backup_root.as_deref(),
    )?;
    let artifact_path = required_path(
        operation.sequence,
        "operations[].artifact_path",
        operation.artifact_path.as_deref(),
    )?;
    let expected = operation.artifact_checksum.as_ref().ok_or(
        RestoreRunnerError::ArtifactStageMissingField {
            sequence: operation.sequence,
            field: "operations[].artifact_checksum",
        },
    )?;
    expected
        .validate()
        .map_err(|source| RestoreRunnerError::ArtifactStageChecksum {
            sequence: operation.sequence,
            source,
        })?;

    let backup_root = validate_backup_root(Path::new(backup_root))?;
    if resolve_backup_artifact_path(&backup_root, artifact_path).is_none() {
        return Err(RestoreRunnerError::ArtifactStagePathConflict {
            path: backup_root.join(artifact_path),
        });
    }

    let stage_root = stage_root(&config.journal)?;
    ensure_private_stage_root(&stage_root)?;
    let operation_root = stage_root.join(format!("operation-{}", operation.sequence));
    remove_stale_operation_root(&operation_root)?;
    create_private_directory(&operation_root)?;
    let staged = StagedRestoreArtifact {
        artifact_path: operation_root.join("artifact"),
        operation_root,
        stage_root,
    };

    let actual = ArtifactChecksum::stage_relative_path_no_follow(
        &backup_root,
        Path::new(artifact_path),
        &staged.artifact_path,
    )
    .map_err(|source| RestoreRunnerError::ArtifactStageChecksum {
        sequence: operation.sequence,
        source,
    })?;
    actual
        .verify(&expected.hash)
        .map_err(|source| RestoreRunnerError::ArtifactStageChecksum {
            sequence: operation.sequence,
            source,
        })?;

    Ok(Some(staged))
}

fn validate_backup_root(path: &Path) -> Result<PathBuf, RestoreRunnerError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|source| stage_io(path.to_path_buf(), source))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(RestoreRunnerError::ArtifactStagePathConflict {
            path: path.to_path_buf(),
        });
    }
    let canonical = path
        .canonicalize()
        .map_err(|source| stage_io(path.to_path_buf(), source))?;
    if !path.is_absolute() || canonical != path {
        return Err(RestoreRunnerError::ArtifactStagePathConflict {
            path: path.to_path_buf(),
        });
    }
    Ok(canonical)
}

pub(super) fn cleanup_pending_upload_stage(
    config: &RestoreRunnerConfig,
    operation: &RestoreApplyJournalOperation,
) -> Result<(), RestoreRunnerError> {
    if operation.operation != RestoreApplyOperationKind::UploadSnapshot {
        return Ok(());
    }
    let root = stage_root(&config.journal)?;
    match fs::symlink_metadata(&root) {
        Ok(metadata) => ensure_private_directory_metadata(&root, &metadata)?,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(stage_io(root, source)),
    }
    remove_stale_operation_root(&root.join(format!("operation-{}", operation.sequence)))?;
    match fs::remove_dir(&root) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::DirectoryNotEmpty => Ok(()),
        Err(source) => Err(stage_io(root, source)),
    }
}

fn required_path<'a>(
    sequence: usize,
    field: &'static str,
    value: Option<&'a str>,
) -> Result<&'a str, RestoreRunnerError> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or(RestoreRunnerError::ArtifactStageMissingField { sequence, field })
}

fn stage_root(journal: &Path) -> Result<PathBuf, RestoreRunnerError> {
    let parent = journal
        .parent()
        .ok_or_else(|| RestoreRunnerError::ArtifactStagePathConflict {
            path: journal.to_path_buf(),
        })?;
    let file_name = journal
        .file_name()
        .ok_or_else(|| RestoreRunnerError::ArtifactStagePathConflict {
            path: journal.to_path_buf(),
        })?
        .to_string_lossy();
    Ok(parent.join(format!(".{file_name}.canic-restore-stage")))
}

fn ensure_private_stage_root(path: &Path) -> Result<(), RestoreRunnerError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => ensure_private_directory_metadata(path, &metadata),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            create_private_directory(path)
        }
        Err(source) => Err(stage_io(path.to_path_buf(), source)),
    }
}

fn ensure_private_directory_metadata(
    path: &Path,
    metadata: &fs::Metadata,
) -> Result<(), RestoreRunnerError> {
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(RestoreRunnerError::ArtifactStagePathConflict {
            path: path.to_path_buf(),
        });
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(RestoreRunnerError::ArtifactStagePathConflict {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn create_private_directory(path: &Path) -> Result<(), RestoreRunnerError> {
    let mut builder = DirBuilder::new();
    #[cfg(unix)]
    builder.mode(0o700);
    builder
        .create(path)
        .map_err(|source| stage_io(path.to_path_buf(), source))
}

fn remove_stale_operation_root(path: &Path) -> Result<(), RestoreRunnerError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            Err(RestoreRunnerError::ArtifactStagePathConflict {
                path: path.to_path_buf(),
            })
        }
        Ok(_) => fs::remove_dir_all(path).map_err(|source| stage_io(path.to_path_buf(), source)),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(stage_io(path.to_path_buf(), source)),
    }
}

const fn stage_io(path: PathBuf, source: std::io::Error) -> RestoreRunnerError {
    RestoreRunnerError::ArtifactStageIo { path, source }
}
