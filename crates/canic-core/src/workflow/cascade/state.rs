//!
//! State snapshot synchronization routines shared by root and child canisters.
//!
//! This module:
//! - cascades snapshots across the subnet topology
//! - applies received snapshots locally on child canisters
//!
//! IMPORTANT:
//! - `StateSnapshotView` is a pure DTO (data only)
//! - All assembly logic lives in `workflow::snapshot`
//! - All persistence happens via ops
//!

use super::warn_if_large;
use crate::{
    Error,
    dto::snapshot::StateSnapshotView,
    log::Topic,
    ops::{
        OpsError,
        prelude::*,
        storage::{
            children::CanisterChildrenOps,
            directory::{AppDirectoryOps, SubnetDirectoryOps},
            registry::SubnetRegistryOps,
            state::{AppStateOps, SubnetStateOps},
        },
    },
    workflow::snapshot::{
        app_directory_data_from_view, app_state_data_from_view, state_snapshot_debug,
        state_snapshot_is_empty, subnet_directory_data_from_view, subnet_state_data_from_view,
    },
};

//
// Cascade logic
//

/// Cascade a state snapshot from the root canister to its direct children.
///
/// No-op if the snapshot is empty.
pub async fn root_cascade_state(snapshot: &StateSnapshotView) -> Result<(), Error> {
    OpsError::require_root()?;

    if state_snapshot_is_empty(snapshot) {
        log!(
            Topic::Sync,
            Info,
            "💦 sync.state: root_cascade skipped (empty snapshot)"
        );
        return Ok(());
    }

    let root_pid = canister_self();
    let children = SubnetRegistryOps::children(root_pid);
    let child_count = children.len();
    warn_if_large("root state cascade", child_count);

    let mut failures = 0;

    for (pid, _) in children {
        if let Err(err) = send_snapshot(&pid, snapshot).await {
            failures += 1;
            log!(
                Topic::Sync,
                Warn,
                "💦 sync.state: failed to cascade to {pid}: {err}",
            );
        }
    }

    if failures > 0 {
        log!(
            Topic::Sync,
            Warn,
            "💦 sync.state: {failures} child cascade(s) failed; continuing"
        );
    }

    Ok(())
}

/// Cascade a snapshot from a non-root canister:
/// - apply it locally
/// - forward it to direct children
pub async fn nonroot_cascade_state(snapshot: &StateSnapshotView) -> Result<(), Error> {
    OpsError::deny_root()?;

    if state_snapshot_is_empty(snapshot) {
        log!(
            Topic::Sync,
            Info,
            "💦 sync.state: nonroot_cascade skipped (empty snapshot)"
        );
        return Ok(());
    }

    // Apply locally first
    apply_state(snapshot)?;

    let children = CanisterChildrenOps::export();
    let child_count = children.len();
    warn_if_large("nonroot state cascade", child_count);

    let mut failures = 0;
    for (pid, _) in children {
        if let Err(err) = send_snapshot(&pid, snapshot).await {
            failures += 1;
            log!(
                Topic::Sync,
                Warn,
                "💦 sync.state: failed to cascade to {pid}: {err}",
            );
        }
    }

    if failures > 0 {
        log!(
            Topic::Sync,
            Warn,
            "💦 sync.state: {failures} child cascade(s) failed; continuing"
        );
    }

    Ok(())
}

//
// Local application
//

/// Apply a received state snapshot locally.
///
/// Only valid on non-root canisters.
fn apply_state(snapshot: &StateSnapshotView) -> Result<(), Error> {
    OpsError::deny_root()?;

    // states
    if let Some(state) = snapshot.app_state {
        AppStateOps::import(app_state_data_from_view(state));
    }
    if let Some(state) = snapshot.subnet_state {
        SubnetStateOps::import(subnet_state_data_from_view(state));
    }

    // directories
    if let Some(dir) = snapshot.app_directory.clone() {
        AppDirectoryOps::import(app_directory_data_from_view(dir));
    }
    if let Some(dir) = snapshot.subnet_directory.clone() {
        SubnetDirectoryOps::import(subnet_directory_data_from_view(dir));
    }

    Ok(())
}

//
// Transport
//

/// Send a state snapshot to another canister.
async fn send_snapshot(pid: &Principal, snapshot: &StateSnapshotView) -> Result<(), Error> {
    let debug = state_snapshot_debug(snapshot);
    log!(Topic::Sync, Info, "💦 sync.state: {debug} -> {pid}");

    call_and_decode::<Result<(), Error>>(*pid, crate::ops::rpc::methods::CANIC_SYNC_STATE, snapshot)
        .await?
}
