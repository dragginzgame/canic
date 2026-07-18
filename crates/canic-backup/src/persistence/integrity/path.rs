//! Module: persistence::integrity::path
//!
//! Responsibility: resolve artifact paths without escaping the backup root.
//! Does not own: artifact verification, filesystem writes, or manifest validation.
//! Boundary: provides path safety checks for persistence integrity code.

use std::path::{Component, Path, PathBuf};

/// Resolve a backup artifact path under the backup root.
#[must_use]
pub fn resolve_backup_artifact_path(root: &Path, artifact_path: &str) -> Option<PathBuf> {
    let path = PathBuf::from(artifact_path);
    if path.is_absolute() {
        return None;
    }
    if path.as_os_str().is_empty() {
        return None;
    }
    let is_safe = path
        .components()
        .all(|component| matches!(component, Component::Normal(_)));
    if !is_safe {
        return None;
    }

    Some(root.join(path))
}
