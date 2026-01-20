//!
//! Topology cascade workflow.
//!
//! Coordinates propagation of topology snapshots from root to leaves.
//! Enforces cascade invariants and delegates transport to `CascadeOps`.

use crate::{
    InternalError, InternalErrorOrigin,
    dto::cascade::TopologySnapshotInput,
    ops::{
        cascade::CascadeOps, ic::IcOps, runtime::env::EnvOps,
        storage::children::CanisterChildrenOps,
    },
    workflow::{
        cascade::{
            snapshot::{
                TopologyPathNode, TopologySnapshot, TopologySnapshotBuilder,
                adapter::TopologySnapshotAdapter,
            },
            warn_if_large,
        },
        prelude::*,
    },
};
use std::collections::HashMap;

///
/// TopologyCascadeWorkflow
/// Orchestrates topology snapshot propagation across the canister tree.
///
pub struct TopologyCascadeWorkflow;

impl TopologyCascadeWorkflow {
    // ───────────────────────── Root cascades ─────────────────────────

    /// Initiates a topology cascade from the root canister toward `target_pid`.
    pub async fn root_cascade_topology_for_pid(target_pid: Principal) -> Result<(), InternalError> {
        EnvOps::require_root()?;

        let snapshot = TopologySnapshotBuilder::for_target(target_pid)?.build();

        let root_pid = IcOps::canister_self();
        let Some(first_child) = Self::next_child_on_path(root_pid, &snapshot.parents)? else {
            log!(
                Topic::Sync,
                Warn,
                "sync.topology: no branch path to {target_pid}, skipping cascade"
            );
            return Ok(());
        };

        let child_snapshot = Self::slice_snapshot_for_child(first_child, &snapshot)?;
        Self::send_snapshot(&first_child, &child_snapshot).await
    }

    // ──────────────────────── Non-root cascades ──────────────────────

    /// Continues a topology cascade on a non-root canister.
    pub async fn nonroot_cascade_topology(
        view: TopologySnapshotInput,
    ) -> Result<(), InternalError> {
        EnvOps::deny_root()?;

        let snapshot = TopologySnapshotAdapter::from_input(view);

        let self_pid = IcOps::canister_self();
        let next = Self::next_child_on_path(self_pid, &snapshot.parents)?;

        let children = snapshot
            .children_map
            .get(&self_pid)
            .cloned()
            .unwrap_or_default();

        warn_if_large("nonroot fanout", children.len());

        let children_entries = children
            .into_iter()
            .map(|child| (child.pid, child.role))
            .collect();

        CanisterChildrenOps::import_direct_children(self_pid, children_entries);

        if let Some(next_pid) = next {
            let next_snapshot = Self::slice_snapshot_for_child(next_pid, &snapshot)?;
            Self::send_snapshot(&next_pid, &next_snapshot).await?;
        }

        Ok(())
    }

    // ───────────────────────── Internal helpers ──────────────────────

    async fn send_snapshot(
        pid: &Principal,
        snapshot: &TopologySnapshot,
    ) -> Result<(), InternalError> {
        let view = TopologySnapshotAdapter::to_input(snapshot);

        CascadeOps::send_topology_snapshot(*pid, &view)
            .await
            .map_err(|err| {
                InternalError::workflow(
                    InternalErrorOrigin::Workflow,
                    format!("topology cascade rejected by child {pid}: {err}"),
                )
            })
    }

    fn next_child_on_path(
        self_pid: Principal,
        parents: &[TopologyPathNode],
    ) -> Result<Option<Principal>, InternalError> {
        let Some(first) = parents.first() else {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "topology parent chain is empty",
            ));
        };

        if first.pid != self_pid {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("topology parent chain does not start with self pid {self_pid}"),
            ));
        }

        Ok(parents.get(1).map(|p| p.pid))
    }

    fn slice_snapshot_for_child(
        next_pid: Principal,
        snapshot: &TopologySnapshot,
    ) -> Result<TopologySnapshot, InternalError> {
        let mut sliced_parents = Vec::new();
        let mut include = false;

        for parent in &snapshot.parents {
            if parent.pid == next_pid {
                include = true;
            }
            if include {
                sliced_parents.push(parent.clone());
            }
        }

        if sliced_parents.is_empty() {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("topology next hop {next_pid} not found in parent chain"),
            ));
        }

        let mut sliced_children_map = HashMap::new();
        for parent in &sliced_parents {
            let children = snapshot
                .children_map
                .get(&parent.pid)
                .cloned()
                .unwrap_or_default();
            sliced_children_map.insert(parent.pid, children);
        }

        Ok(TopologySnapshot {
            parents: sliced_parents,
            children_map: sliced_children_map,
        })
    }
}
