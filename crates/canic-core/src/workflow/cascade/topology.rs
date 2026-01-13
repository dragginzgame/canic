//!
//! Topology cascade workflow.
//!
//! Coordinates propagation of topology snapshots from root to leaves.
//! Enforces cascade invariants and delegates transport to `CascadeOps`.

use crate::{
    InternalError, InternalErrorOrigin, access,
    dto::cascade::TopologySnapshotView,
    ops::{
        cascade::CascadeOps,
        ic::IcOps,
        storage::{
            CanisterRecord,
            children::{CanisterChildrenOps, ChildrenSnapshot},
        },
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
        access::env::require_root()?;

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
    pub async fn nonroot_cascade_topology(view: TopologySnapshotView) -> Result<(), InternalError> {
        access::env::deny_root()?;

        let snapshot = TopologySnapshotAdapter::from_view(view);

        let self_pid = IcOps::canister_self();
        let next = Self::next_child_on_path(self_pid, &snapshot.parents)?;

        let children = snapshot
            .children_map
            .get(&self_pid)
            .cloned()
            .unwrap_or_default();

        warn_if_large("nonroot fanout", children.len());

        // Build children cache snapshot using unified CanisterRecord.
        // Note: cached entries may have empty `module_hash` / `created_at`
        // fields; canonical data lives in the registry.
        let children_snapshot = ChildrenSnapshot {
            entries: children
                .into_iter()
                .map(|child| {
                    (
                        child.pid,
                        CanisterRecord {
                            role: child.role,
                            parent_pid: Some(self_pid),
                            module_hash: None,
                            created_at: 0,
                        },
                    )
                })
                .collect(),
        };

        // Invariant: children cache is updated only via topology cascade.
        CanisterChildrenOps::import(children_snapshot);

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
        let view = TopologySnapshotAdapter::to_view(snapshot);

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
