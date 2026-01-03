//! Topology synchronization helpers.

use super::{
    CascadeError,
    snapshot::{TopologyPathNode, TopologySnapshot, TopologySnapshotBuilder},
    warn_if_large,
};
use crate::workflow::cascade::snapshot::adapter::topology_snapshot_from_view;
use crate::{
    Error, access,
    dto::cascade::TopologySnapshotView,
    ops::{
        self,
        storage::children::{CanisterChildrenOps, ChildSnapshot, ChildrenSnapshot},
    },
    workflow::prelude::*,
};
use std::collections::HashMap;

//
// ===========================================================================
//  ROOT CASCADES
// ===========================================================================
//

pub async fn root_cascade_topology_for_pid(target_pid: Principal) -> Result<(), Error> {
    access::env::require_root()?;

    let snapshot = TopologySnapshotBuilder::for_target(target_pid)?.build();

    let root_pid = canister_self();
    let Some(first_child) = next_child_on_path(root_pid, &snapshot.parents)? else {
        log!(
            Topic::Sync,
            Warn,
            "sync.topology: no branch path to {target_pid}, skipping cascade"
        );
        return Ok(());
    };

    let child_snapshot = slice_snapshot_for_child(first_child, &snapshot)?;
    send_snapshot(&first_child, &child_snapshot).await?;

    Ok(())
}

//
// ===========================================================================
//  NON-ROOT CASCADES
// ===========================================================================
//

pub async fn nonroot_cascade_topology(view: TopologySnapshotView) -> Result<(), Error> {
    access::env::deny_root()?;

    let snapshot = topology_snapshot_from_view(view);
    let self_pid = canister_self();
    let next = next_child_on_path(self_pid, &snapshot.parents)?;

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

    // Invariant: children cache updated only via topology cascade
    CanisterChildrenOps::import(children_snapshot);

    if let Some(next_pid) = next {
        let next_snapshot = slice_snapshot_for_child(next_pid, &snapshot)?;
        send_snapshot(&next_pid, &next_snapshot).await?;
    }

    Ok(())
}

//
// ===========================================================================
//  HELPERS
// ===========================================================================
//

async fn send_snapshot(pid: &Principal, snapshot: &TopologySnapshot) -> Result<(), Error> {
    let view = TopologySnapshotView::from(snapshot);

    ops::rpc::cascade::send_topology_snapshot(*pid, &view)
        .await
        .map_err(|_| CascadeError::ChildRejected(*pid).into())
}

//
// ===========================================================================
//  PATH HELPERS (internal-only)
// ===========================================================================
//

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
