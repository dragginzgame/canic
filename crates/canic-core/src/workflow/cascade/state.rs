//!
//! State cascade workflow.
//!
//! Coordinates propagation of internal state snapshots across the subnet topology.
//! Root canisters initiate cascades; non-root canisters apply and forward snapshots.
//!
//! Layering rules:
//! - Workflow operates on `StateSnapshot` (internal)
//! - `StateSnapshotInput` is used only for transport (RPC / API)
//! - Snapshot assembly lives in `workflow::cascade::snapshot`
//! - Persistence and mutation live in ops

use crate::{
    InternalError, InternalErrorOrigin,
    dto::cascade::StateSnapshotInput,
    ops::{
        cascade::CascadeOps,
        ic::IcOps,
        runtime::env::EnvOps,
        storage::{
            children::CanisterChildrenOps,
            directory::{app::AppDirectoryOps, subnet::SubnetDirectoryOps},
            registry::subnet::SubnetRegistryOps,
            state::{app::AppStateOps, subnet::SubnetStateOps},
        },
    },
    workflow::{
        cascade::{
            snapshot::{
                StateSnapshot, adapter::StateSnapshotAdapter, state_snapshot_debug,
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
    pub async fn root_cascade_state(snapshot: &StateSnapshot) -> Result<(), InternalError> {
        EnvOps::require_root()?;

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

        let root_pid = IcOps::canister_self();
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

    /// Cascade a state snapshot only through the branch that contains `target_pid`.
    ///
    /// This is used for create/adopt flows where only one subtree needs the
    /// current state projection to reach a newly attached descendant.
    pub async fn root_cascade_state_for_pid(
        target_pid: Principal,
        snapshot: &StateSnapshot,
    ) -> Result<(), InternalError> {
        EnvOps::require_root()?;

        if state_snapshot_is_empty(snapshot) {
            log!(
                Topic::Sync,
                Info,
                "sync.state: targeted root cascade skipped (empty snapshot)"
            );
            return Ok(());
        }

        log!(
            Topic::Sync,
            Info,
            "sync.state: targeted root cascade start target={target_pid} snapshot={}",
            state_snapshot_debug(snapshot)
        );

        let root_pid = IcOps::canister_self();
        let parent_chain = SubnetRegistryOps::parent_chain(target_pid)?;
        let Some(next_child) = next_child_on_path(root_pid, &parent_chain)? else {
            log!(
                Topic::Sync,
                Warn,
                "sync.state: no branch path to {target_pid}, skipping targeted cascade"
            );
            return Ok(());
        };

        Self::send_snapshot(next_child, snapshot).await
    }

    // ──────────────────────── Non-root cascade ──────────────────────

    /// Handle a received state snapshot on a non-root canister:
    /// - apply it locally
    /// - forward it to direct children using the children cache
    pub async fn nonroot_cascade_state(view: StateSnapshotInput) -> Result<(), InternalError> {
        EnvOps::deny_root()?;

        let snapshot = StateSnapshotAdapter::from_input(view);

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
    fn apply_state(snapshot: &StateSnapshot) -> Result<(), InternalError> {
        EnvOps::deny_root()?;

        if let Some(app) = snapshot.app_state {
            AppStateOps::import_input(app);
        }

        if let Some(subnet) = snapshot.subnet_state {
            SubnetStateOps::import_input(subnet);
        }

        if let Some(dir) = &snapshot.app_directory {
            AppDirectoryOps::import_args_allow_incomplete(dir.clone())?;
        }

        if let Some(dir) = &snapshot.subnet_directory {
            SubnetDirectoryOps::import_args_allow_incomplete(dir.clone())?;
        }

        Ok(())
    }

    // ───────────────────────── Transport ────────────────────────────

    /// Send a state snapshot to another canister.
    ///
    /// Converts internal snapshot → DTO exactly once.
    async fn send_snapshot(pid: Principal, snapshot: &StateSnapshot) -> Result<(), InternalError> {
        let view = StateSnapshotAdapter::to_input(snapshot);

        CascadeOps::send_state_snapshot(pid, &view)
            .await
            .map_err(|err| {
                InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!("state cascade rejected by child {pid}: {err}"),
                )
            })
    }
}

// Resolve the first child hop from `self_pid` toward `target_pid`.
fn next_child_on_path(
    self_pid: Principal,
    parents: &[(Principal, crate::storage::canister::CanisterRecord)],
) -> Result<Option<Principal>, InternalError> {
    let Some((first_pid, _)) = parents.first() else {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "state parent chain is empty",
        ));
    };

    if *first_pid != self_pid {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            format!("state parent chain does not start with self pid {self_pid}"),
        ));
    }

    Ok(parents.get(1).map(|(pid, _)| *pid))
}

#[cfg(test)]
mod tests {
    use super::next_child_on_path;
    use crate::{cdk::types::Principal, storage::canister::CanisterRecord};

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn record(parent_pid: Option<Principal>) -> CanisterRecord {
        CanisterRecord {
            role: crate::ids::CanisterRole::new("state_path_test"),
            parent_pid,
            module_hash: None,
            created_at: 0,
        }
    }

    #[test]
    fn next_child_on_path_returns_first_branch_child() {
        let chain = vec![
            (p(1), record(None)),
            (p(2), record(Some(p(1)))),
            (p(3), record(Some(p(2)))),
        ];

        assert_eq!(next_child_on_path(p(1), &chain).unwrap(), Some(p(2)));
    }

    #[test]
    fn next_child_on_path_returns_none_for_root_target() {
        let chain = vec![(p(1), record(None))];

        assert_eq!(next_child_on_path(p(1), &chain).unwrap(), None);
    }
}
