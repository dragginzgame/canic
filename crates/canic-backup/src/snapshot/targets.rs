use super::{SnapshotDownloadConfig, SnapshotDownloadError, SnapshotDriver};
use crate::discovery::{SnapshotTarget, targets_from_registry};

/// Resolve the selected canister plus optional direct/recursive children.
pub fn resolve_snapshot_targets(
    config: &SnapshotDownloadConfig,
    driver: &mut impl SnapshotDriver,
) -> Result<Vec<SnapshotTarget>, SnapshotDownloadError> {
    if !config.include_children {
        return Ok(vec![SnapshotTarget {
            canister_id: config.canister.clone(),
            role: None,
            parent_canister_id: None,
            module_hash: None,
        }]);
    }

    let registry = if let Some(root) = &config.root {
        driver
            .registry_entries(root)
            .map_err(SnapshotDownloadError::Driver)?
    } else {
        return Err(SnapshotDownloadError::MissingRegistrySource);
    };
    targets_from_registry(&registry, &config.canister, config.recursive)
        .map_err(SnapshotDownloadError::from)
}
