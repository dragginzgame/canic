//!
//! State snapshot synchronization routines shared by root and child canisters.
//!
//! This module:
//! - cascades internal state snapshots across the subnet topology
//! - applies received snapshots locally on child canisters
//!
//! LAYERING RULES:
//! - Workflow operates on `StateSnapshot` (internal)
//! - `StateSnapshotView` is used only for transport (RPC / API)
//! - Assembly lives in `workflow::cascade::snapshot`
//! - Persistence lives in ops
//!

use super::{
    CascadeError,
    snapshot::{StateSnapshot, state_snapshot_is_empty},
    warn_if_large,
};
use crate::workflow::cascade::snapshot::adapter::state_snapshot_from_view;
use crate::{
    Error,
    access::env,
    dto::cascade::StateSnapshotView,
    ops::{
        self,
        storage::{
            children::CanisterChildrenOps,
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            registry::subnet::SubnetRegistryOps,
            state::{app::AppStateOps, subnet::SubnetStateOps},
        },
    },
    workflow::prelude::*,
};

//
// ROOT CASCADE
//

/// Cascade a state snapshot from the root canister to its direct children.
///
/// No-op if the snapshot is empty.
pub(crate) async fn root_cascade_state(snapshot: &StateSnapshot) -> Result<(), Error> {
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
    let children = SubnetRegistryOps::children(root_pid);
    warn_if_large("root state cascade", children.len());

    let mut failures = 0;

    for (pid, _) in children {
        if let Err(err) = send_snapshot(pid, snapshot).await {
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
// NON-ROOT CASCADE
//

/// Cascade a snapshot from a non-root canister:
/// - apply it locally
/// - forward it to direct children (from children cache)
pub(crate) async fn nonroot_cascade_state(view: StateSnapshotView) -> Result<(), Error> {
    env::deny_root()?;

    let snapshot = state_snapshot_from_view(view)?;

    if state_snapshot_is_empty(&snapshot) {
        log!(
            Topic::Sync,
            Info,
            "💦 sync.state: nonroot_cascade skipped (empty snapshot)"
        );
        return Ok(());
    }

    // Apply locally first
    apply_state(&snapshot)?;

    // Cascade using children cache (never registry)
    let child_pids = CanisterChildrenOps::pids();
    warn_if_large("nonroot state cascade", child_pids.len());

    let mut failures = 0;

    for pid in child_pids {
        if let Err(err) = send_snapshot(pid, &snapshot).await {
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
// LOCAL APPLICATION
//

/// Apply a received state snapshot locally.
///
/// Only valid on non-root canisters.
fn apply_state(snapshot: &StateSnapshot) -> Result<(), Error> {
    env::deny_root()?;

    if let Some(app) = &snapshot.app_state {
        AppStateOps::import(app.clone())?;
    }

    if let Some(subnet) = &snapshot.subnet_state {
        SubnetStateOps::import(subnet.clone());
    }

    if let Some(dir) = &snapshot.app_directory {
        AppDirectoryOps::import(dir.clone());
    }

    if let Some(dir) = &snapshot.subnet_directory {
        SubnetDirectoryOps::import(dir.clone());
    }

    Ok(())
}

//
// TRANSPORT
//

/// Send a state snapshot to another canister.
///
/// Converts internal snapshot → DTO exactly once.
async fn send_snapshot(pid: Principal, snapshot: &StateSnapshot) -> Result<(), Error> {
    let view = StateSnapshotView::from(snapshot);

    ops::rpc::cascade::send_state_snapshot(pid, &view)
        .await
        .map_err(|_| CascadeError::ChildRejected(pid).into())
}
