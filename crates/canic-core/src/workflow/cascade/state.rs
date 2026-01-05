//! State cascade workflow.
//!
//! Coordinates propagation of internal state snapshots across the subnet topology.
//! Root canisters initiate cascades; non-root canisters apply and forward snapshots.
//!
//! Layering rules:
//! - Workflow operates on `StateSnapshot` (internal)
//! - `StateSnapshotView` is used only for transport (RPC / API)
//! - Snapshot assembly lives in `workflow::cascade::snapshot`
//! - Persistence and mutation live in ops

use crate::{
    Error, access,
    dto::cascade::StateSnapshotView,
    ops::{
        cascade::CascadeOps,
        storage::{
            children::CanisterChildrenOps,
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            registry::subnet::SubnetRegistryOps,
            state::{app::AppStateOps, subnet::SubnetStateOps},
        },
    },
    workflow::{
        cascade::{
            CascadeError,
            snapshot::{
                StateSnapshot, adapter::state_snapshot_from_view, state_snapshot_debug,
                state_snapshot_is_empty,
            },
            warn_if_large,
        },
        prelude::*,
    },
};

///
/// StateCascadeWorkflow
/// Orchestrates state snapshot propagation and local application.
///

pub struct StateCascadeWorkflow;

impl StateCascadeWorkflow {
    // ───────────────────────── Root cascade ─────────────────────────

    /// Cascade a state snapshot from the root canister to its direct children.
    ///
    /// No-op if the snapshot is empty.
    pub async fn root_cascade_state(snapshot: &StateSnapshot) -> Result<(), Error> {
        access::env::require_root()?;

        if state_snapshot_is_empty(snapshot) {
            log!(
                Topic::Sync,
                Info,
                "sync.state: root cascade skipped (empty snapshot)"
            );
            return Ok(());
        }

        log!(
            Topic::Sync,
            Info,
            "sync.state: root cascade start snapshot={}",
            state_snapshot_debug(snapshot)
        );

        let root_pid = canister_self();
        let children = SubnetRegistryOps::children(root_pid);
        warn_if_large("root state cascade", children.len());

        let mut failures = 0;

        for (pid, _) in children {
            if let Err(err) = Self::send_snapshot(pid, snapshot).await {
                failures += 1;
                log!(
                    Topic::Sync,
                    Warn,
                    "sync.state: failed to cascade to {pid}: {err}",
                );
            }
        }

        if failures > 0 {
            log!(
                Topic::Sync,
                Warn,
                "sync.state: {failures} child cascade(s) failed; continuing"
            );
        }

        Ok(())
    }

    // ──────────────────────── Non-root cascade ──────────────────────

    /// Handle a received state snapshot on a non-root canister:
    /// - apply it locally
    /// - forward it to direct children using the children cache
    pub async fn nonroot_cascade_state(view: StateSnapshotView) -> Result<(), Error> {
        access::env::deny_root()?;

        let snapshot = state_snapshot_from_view(view);

        if state_snapshot_is_empty(&snapshot) {
            log!(
                Topic::Sync,
                Info,
                "sync.state: non-root cascade skipped (empty snapshot)"
            );
            return Ok(());
        }

        log!(
            Topic::Sync,
            Info,
            "sync.state: non-root cascade start snapshot={}",
            state_snapshot_debug(&snapshot)
        );

        // Apply locally before forwarding.
        Self::apply_state(&snapshot)?;

        // Cascade using children cache only (never registry).
        let child_pids = CanisterChildrenOps::pids();
        warn_if_large("non-root state cascade", child_pids.len());

        let mut failures = 0;

        for pid in child_pids {
            if let Err(err) = Self::send_snapshot(pid, &snapshot).await {
                failures += 1;
                log!(
                    Topic::Sync,
                    Warn,
                    "sync.state: failed to cascade to {pid}: {err}",
                );
            }
        }

        if failures > 0 {
            log!(
                Topic::Sync,
                Warn,
                "sync.state: {failures} child cascade(s) failed; continuing"
            );
        }

        Ok(())
    }

    // ─────────────────────── Local application ──────────────────────

    /// Apply a received state snapshot locally.
    ///
    /// Valid only on non-root canisters.
    fn apply_state(snapshot: &StateSnapshot) -> Result<(), Error> {
        access::env::deny_root()?;

        if let Some(app) = &snapshot.app_state {
            AppStateOps::import(app.clone())?;
        }

        if let Some(subnet_snapshot) = &snapshot.subnet_state {
            SubnetStateOps::import(subnet_snapshot.clone());
        }

        if let Some(dir) = &snapshot.app_directory {
            AppDirectoryOps::import(dir.clone());
        }

        if let Some(dir) = &snapshot.subnet_directory {
            SubnetDirectoryOps::import(dir.clone());
        }

        Ok(())
    }

    // ───────────────────────── Transport ────────────────────────────

    /// Send a state snapshot to another canister.
    ///
    /// Converts internal snapshot → DTO exactly once.
    async fn send_snapshot(pid: Principal, snapshot: &StateSnapshot) -> Result<(), Error> {
        let view = StateSnapshotView::from(snapshot);

        CascadeOps::send_state_snapshot(pid, &view)
            .await
            .map_err(|_| CascadeError::ChildRejected(pid).into())
    }
}
