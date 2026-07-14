//! Module: runner::operations::path
//!
//! Responsibility: derive and validate runner artifact paths.
//! Does not own: filesystem writes, checksum verification, or backup layout persistence.
//! Boundary: keeps operation temp paths bound to the configured backup root.

use crate::{
    artifacts::artifact_path_segment, persistence::BackupLayout, runner::BackupRunnerError,
};

use std::path::{Path, PathBuf};

pub(super) fn artifact_relative_path(canister_id: &str) -> String {
    artifact_path_segment(canister_id)
}

pub(super) fn artifact_temp_path(root: &Path, canister_id: &str) -> PathBuf {
    root.join(format!("{}.tmp", artifact_path_segment(canister_id)))
}

pub(super) fn ensure_expected_temp_path(
    layout: &BackupLayout,
    sequence: usize,
    target: &str,
    temp_path: &str,
) -> Result<(), BackupRunnerError> {
    let expected = artifact_temp_path(layout.root(), target);
    if Path::new(temp_path) != expected {
        return Err(BackupRunnerError::ArtifactTempPathMismatch {
            sequence,
            target_canister_id: target.to_string(),
            journal_path: temp_path.to_string(),
            expected_path: expected.display().to_string(),
        });
    }
    Ok(())
}
