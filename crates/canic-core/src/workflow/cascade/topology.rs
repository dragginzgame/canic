//! Topology cascade workflow.
//!
//! Coordinates propagation of topology snapshots from root to leaves.
//! Enforces cascade invariants and delegates transport to `CascadeOps`.

use crate::{
    Error, access,
    dto::cascade::TopologySnapshotView,
    ops::{
        cascade::CascadeOps,
        storage::children::{CanisterChildrenOps, ChildSnapshot, ChildrenSnapshot},
    },
    workflow::{
        cascade::{
            CascadeError,
            snapshot::{
                TopologyPathNode, TopologySnapshot, TopologySnapshotBuilder,
                adapter::topology_snapshot_from_view,
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
    pub async fn root_cascade_topology_for_pid(target_pid: Principal) -> Result<(), Error> {
        access::env::require_root()?;

        let snapshot = TopologySnapshotBuilder::for_target(target_pid)?.build();

        let root_pid = canister_self();
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
    pub async fn nonroot_cascade_topology(view: TopologySnapshotView) -> Result<(), Error> {
        access::env::deny_root()?;

        let snapshot = topology_snapshot_from_view(view);
        let self_pid = canister_self();
        let next = Self::next_child_on_path(self_pid, &snapshot.parents)?;

        let children = snapshot
            .children_map
            .get(&self_pid)
            .cloned()
            .unwrap_or_default();

        warn_if_large("nonroot fanout", children.len());

        let children_snapshot = ChildrenSnapshot {
            entries: children
                .into_iter()
                .map(|child| ChildSnapshot {
                    pid: child.pid,
                    role: child.role,
                    parent_pid: Some(self_pid),
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

    async fn send_snapshot(pid: &Principal, snapshot: &TopologySnapshot) -> Result<(), Error> {
        let view = TopologySnapshotView::from(snapshot);

        CascadeOps::send_topology_snapshot(*pid, &view)
            .await
            .map_err(|_| CascadeError::ChildRejected(*pid).into())
    }

    fn next_child_on_path(
        self_pid: Principal,
        parents: &[TopologyPathNode],
    ) -> Result<Option<Principal>, Error> {
        let Some(first) = parents.first() else {
            return Err(CascadeError::InvalidParentChain.into());
        };

        if first.pid != self_pid {
            return Err(CascadeError::ParentChainMissingSelf(self_pid).into());
        }

        Ok(parents.get(1).map(|p| p.pid))
    }

    fn slice_snapshot_for_child(
        next_pid: Principal,
        snapshot: &TopologySnapshot,
    ) -> Result<TopologySnapshot, Error> {
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
            return Err(CascadeError::NextHopNotFound(next_pid).into());
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
