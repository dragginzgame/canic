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

use crate::{
    Error,
    access::env,
    dto::snapshot::StateSnapshotView,
    ops::{
        self,
        storage::{
            children::CanisterChildrenOps,
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            registry::{adapter::app_registry_view_from_snapshot, subnet::SubnetRegistryOps},
            state::{app::AppStateOps, subnet::SubnetStateOps},
        },
    },
    workflow::{
        cascade::{CascadeError, warn_if_large},
        prelude::*,
        snapshot::state_snapshot_is_empty,
        state::adapter::{app_state_snapshot_from_view, subnet_state_snapshot_from_view},
    },
};

//
// Cascade logic
//

/// Cascade a state snapshot from the root canister to its direct children.
///
/// No-op if the snapshot is empty.
pub(crate) async fn root_cascade_state(snapshot: &StateSnapshotView) -> Result<(), Error> {
    env::require_root()?;

    if state_snapshot_is_empty(snapshot) {
        log!(
            Topic::Sync,
            Info,
            "💦 sync.state: root_cascade skipped (empty snapshot)"
        );
        return Ok(());
    }

    let root_pid = canister_self();
    let children = SubnetRegistryOps::children_view(root_pid);
    let child_count = children.len();
    warn_if_large("root state cascade", child_count);

    let mut failures = 0;

    for child in children {
        let pid = child.pid;
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
pub(crate) async fn nonroot_cascade_state(snapshot: &StateSnapshotView) -> Result<(), Error> {
    env::deny_root()?;

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

    let child_pids = CanisterChildrenOps::pids();
    let child_count = child_pids.len();
    warn_if_large("nonroot state cascade", child_count);

    let mut failures = 0;
    for pid in child_pids {
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
    env::deny_root()?;

    if let Some(view) = snapshot.app_state {
        let snap = app_state_snapshot_from_view(view);
        AppStateOps::import(snap)?;
    }

    if let Some(view) = snapshot.subnet_state {
        let snap = subnet_state_snapshot_from_view(view);
        SubnetStateOps::import(snap);
    }

    if let Some(view) = snapshot.app_directory {
        let snap = app_directory_snapshot_from_view(view);
        AppDirectoryOps::import(snap);
    }

    if let Some(view) = snapshot.subnet_directory {
        let snap = subnet_directory_snapshot_from_view(view);
        SubnetDirectoryOps::import(snap);
    }

    Ok(())
}

//
// Transport
//

/// Send a state snapshot to another canister.
async fn send_snapshot(pid: &Principal, snapshot: &StateSnapshotView) -> Result<(), Error> {
    ops::rpc::cascade::send_state_snapshot(*pid, snapshot)
        .await
        .map_err(|_| CascadeError::ChildRejected(*pid).into())
}
