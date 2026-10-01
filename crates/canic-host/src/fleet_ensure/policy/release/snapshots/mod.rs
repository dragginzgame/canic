//! Exact snapshot-removal reconciliation under independent operator custody.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::model::{CanisterRuntimeStatus, EnsureAction, LiveCanister};
use thiserror::Error;

/// Refuse ambiguous destructive intent or changed physical custody/inventory.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum SnapshotRemovalError {
    #[error("snapshot removal requires the reviewed stopped canister under sole operator custody")]
    Custody,
    #[error("snapshot removal inventory differs from the exact reviewed before/after set")]
    Inventory,
}

/// Recognize the exact before/after inventory. The executor owns durable intent authority.
pub(in crate::fleet_ensure) fn reconcile(
    action: &EnsureAction,
    operator: &str,
    live: Option<&LiveCanister>,
    snapshots: &[String],
) -> Result<bool, SnapshotRemovalError> {
    let EnsureAction::DeleteSnapshot {
        principal,
        snapshot_id,
        expected_snapshots,
        expected_module_sha256,
        ..
    } = action
    else {
        return Err(SnapshotRemovalError::Inventory);
    };
    let valid_id = |id: &String| {
        !id.is_empty()
            && id.len().is_multiple_of(2)
            && id
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    };
    if !expected_snapshots.iter().all(valid_id)
        || !expected_snapshots.windows(2).all(|pair| pair[0] < pair[1])
        || !expected_snapshots.contains(snapshot_id)
    {
        return Err(SnapshotRemovalError::Inventory);
    }
    let Some(live) = live else {
        return Err(SnapshotRemovalError::Custody);
    };
    if live.principal != *principal
        || live.status != CanisterRuntimeStatus::Stopped
        || live.controllers != [operator]
        || live.module_sha256 != *expected_module_sha256
    {
        return Err(SnapshotRemovalError::Custody);
    }
    if snapshots == expected_snapshots {
        return Ok(false);
    }
    let remaining = expected_snapshots.iter().filter(|id| *id != snapshot_id);
    if remaining.eq(snapshots.iter()) {
        return Ok(true);
    }
    Err(SnapshotRemovalError::Inventory)
}
