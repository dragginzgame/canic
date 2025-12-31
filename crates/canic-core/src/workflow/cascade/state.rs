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
    Error, PublicError,
    dto::snapshot::StateSnapshotView,
    ops::{
        ic::mgmt::call_and_decode,
        runtime::env::EnvOps,
        storage::{
            children::CanisterChildrenOps,
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            registry::subnet::SubnetRegistryOps,
            state::{app::AppStateOps, subnet::SubnetStateOps},
        },
    },
    workflow::{
        cascade::{CascadeError, warn_if_large},
        prelude::*,
        snapshot::{state_snapshot_debug, state_snapshot_is_empty},
    },
};

//
// Cascade logic
//

/// Cascade a state snapshot from the root canister to its direct children.
///
/// No-op if the snapshot is empty.
pub(crate) async fn root_cascade_state(snapshot: &StateSnapshotView) -> Result<(), Error> {
    EnvOps::require_root()?;

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
pub(crate) async fn nonroot_cascade_state_internal(
    snapshot: &StateSnapshotView,
) -> Result<(), Error> {
    EnvOps::deny_root()?;

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

pub async fn nonroot_cascade_state(snapshot: &StateSnapshotView) -> Result<(), PublicError> {
    nonroot_cascade_state_internal(snapshot)
        .await
        .map_err(PublicError::from)
}

//
// Local application
//

/// Apply a received state snapshot locally.
///
/// Only valid on non-root canisters.
fn apply_state(snapshot: &StateSnapshotView) -> Result<(), Error> {
    EnvOps::deny_root()?;

    // states
    if let Some(state) = snapshot.app_state {
        AppStateOps::import_view(state);
    }
    if let Some(state) = snapshot.subnet_state {
        SubnetStateOps::import_view(state);
    }

    // directories
    if let Some(dir) = snapshot.app_directory.clone() {
        AppDirectoryOps::import_view(dir);
    }
    if let Some(dir) = snapshot.subnet_directory.clone() {
        SubnetDirectoryOps::import_view(dir);
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

    let result = call_and_decode::<Result<(), PublicError>>(
        *pid,
        crate::ops::rpc::methods::CANIC_SYNC_STATE,
        snapshot,
    )
    .await?;

    // Boundary: convert PublicError from child call into internal Error.
    result.map_err(|err| CascadeError::ChildRejected(err).into())
}
