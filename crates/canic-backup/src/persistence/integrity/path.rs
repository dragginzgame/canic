//! Module: persistence::integrity::path
//!
//! Responsibility: resolve artifact paths without escaping the backup root.
//! Does not own: artifact verification, filesystem writes, or manifest validation.
//! Boundary: provides path safety checks for persistence integrity code.

use std::path::{Path, PathBuf};

/// Resolve a backup artifact path under the backup root.
#[must_use]
pub fn resolve_backup_artifact_path(root: &Path, artifact_path: &str) -> Option<PathBuf> {
    crate::artifacts::relative_artifact_path(artifact_path).map(|path| root.join(path))
}
