//! Topology synchronization helpers.
//!
//! Propagates topology snapshots along a targeted branch.
//!
//! Snapshots carry a trimmed parent chain and the per-node direct children for
//! that chain only. Each hop receives a suffix of the chain (starting at self),
//! validates it (cycle/root termination checks), imports its direct children,
//! and forwards a further-trimmed snapshot to the next hop. Failures are logged
//! and abort the cascade rather than continuing with partial data.

use crate::{
    Error, PublicError,
    dto::snapshot::{TopologyNodeView, TopologySnapshotView},
    ops::{
        ic::mgmt::call_and_decode, runtime::env::EnvOps, storage::children::CanisterChildrenOps,
    },
    workflow::{
        cascade::{CascadeError, warn_if_large},
        prelude::*,
        snapshot::TopologySnapshotBuilder,
    },
};
use std::collections::HashMap;

//
// ===========================================================================
//  ROOT CASCADES
// ===========================================================================
//

pub(crate) async fn root_cascade_topology_for_pid(target_pid: Principal) -> Result<(), Error> {
    EnvOps::require_root()?;

    let snapshot = match TopologySnapshotBuilder::for_target(target_pid) {
        Ok(builder) => builder.build(),
        Err(err) => {
            log!(
                Topic::Sync,
                Error,
                "sync.topology: failed to build snapshot for target {target_pid}: {err}"
            );
            return Err(err);
        }
    };
    let root_pid = canister_self();
    let Some(first_child) = (match next_child_on_path(root_pid, &snapshot.parents) {
        Ok(next) => next,
        Err(err) => {
            log!(
                Topic::Sync,
                Error,
                "sync.topology: invalid parent chain for {target_pid}: {err}"
            );
            return Err(err);
        }
    }) else {
        log!(
            Topic::Sync,
            Warn,
            "sync.topology: no branch path to {target_pid}, skipping targeted cascade"
        );

        return Ok(());
    };

    let child_snapshot = match slice_snapshot_for_child(first_child, &snapshot) {
        Ok(s) => s,
        Err(err) => {
            log!(
                Topic::Sync,
                Error,
                "sync.topology: failed to slice snapshot for child {first_child}: {err}"
            );
            return Err(err);
        }
    };
    if let Err(err) = send_snapshot(&first_child, &child_snapshot).await {
        log!(
            Topic::Sync,
            Warn,
            "sync.topology: failed targeted cascade to first child {first_child}: {err}"
        );
    } else {
        log!(
            Topic::Sync,
            Info,
            "sync.topology: delegated targeted cascade to {first_child}"
        );
    }

    Ok(())
}

//
// ===========================================================================
//  NON-ROOT CASCADES
// ===========================================================================
//

pub(crate) async fn nonroot_cascade_topology_internal(
    snapshot: &TopologySnapshotView,
) -> Result<(), Error> {
    EnvOps::deny_root()?;

    let self_pid = canister_self();
    let next = match next_child_on_path(self_pid, &snapshot.parents) {
        Ok(next) => next,
        Err(err) => {
            log!(
                Topic::Sync,
                Error,
                "sync.topology: rejecting snapshot for {self_pid} (invalid parent chain len={}): {err}",
                snapshot.parents.len()
            );
            return Err(err);
        }
    };

    let children = snapshot
        .children_map
        .get(&self_pid)
        .cloned()
        .unwrap_or_default();
    warn_if_large("nonroot fanout", children.len());
    CanisterChildrenOps::import_view(self_pid, children);

    if let Some(next_pid) = next {
        let next_snapshot = match slice_snapshot_for_child(next_pid, snapshot) {
            Ok(s) => s,
            Err(err) => {
                log!(
                    Topic::Sync,
                    Error,
                    "sync.topology: failed to slice snapshot for child {next_pid}: {err}"
                );
                return Err(err);
            }
        };
        send_snapshot(&next_pid, &next_snapshot).await?;
    }

    Ok(())
}

pub async fn nonroot_cascade_topology(snapshot: &TopologySnapshotView) -> Result<(), PublicError> {
    nonroot_cascade_topology_internal(snapshot)
        .await
        .map_err(PublicError::from)
}

//
// ===========================================================================
//  HELPERS
// ===========================================================================
//

async fn send_snapshot(pid: &Principal, snapshot: &TopologySnapshotView) -> Result<(), Error> {
    let result = call_and_decode::<Result<(), PublicError>>(
        *pid,
        crate::ops::rpc::methods::CANIC_SYNC_TOPOLOGY,
        snapshot,
    )
    .await?;

    // Boundary: convert PublicError from child call into internal Error.
    result.map_err(|err| CascadeError::ChildRejected(err).into())
}

//
// ===========================================================================
//  PATH HELPERS
// ===========================================================================
//

fn next_child_on_path(
    self_pid: Principal,
    parents: &[TopologyNodeView],
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
    snapshot: &TopologySnapshotView,
) -> Result<TopologySnapshotView, Error> {
    // Slice parents chain so we start at the next hop
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

    // Slice children_map so it includes only nodes in the sliced chain
    let mut sliced_children_map = HashMap::new();
    for parent in &sliced_parents {
        let children = snapshot
            .children_map
            .get(&parent.pid)
            .cloned()
            .unwrap_or_default();
        sliced_children_map.insert(parent.pid, children);
    }

    Ok(TopologySnapshotView {
        parents: sliced_parents,
        children_map: sliced_children_map,
    })
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dto::snapshot::TopologyChildView, ids::CanisterRole};

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn n(pid: Principal, parent_pid: Option<Principal>) -> TopologyNodeView {
        TopologyNodeView {
            pid,
            role: CanisterRole::new("test"),
            parent_pid,
        }
    }

    fn c(pid: Principal) -> TopologyChildView {
        TopologyChildView {
            pid,
            role: CanisterRole::new("test"),
        }
    }

    #[test]
    fn next_child_from_chain() {
        let root = p(1);
        let hub = p(2);
        let inst = p(3);
        let ledg = p(4);

        let parents = vec![
            n(root, None),
            n(hub, Some(root)),
            n(inst, Some(hub)),
            n(ledg, Some(inst)),
        ];

        assert_eq!(next_child_on_path(root, &parents).unwrap(), Some(hub));
        assert_eq!(next_child_on_path(hub, &parents[1..]).unwrap(), Some(inst));
        assert_eq!(next_child_on_path(ledg, &parents[3..]).unwrap(), None);
    }

    #[test]
    fn slice_snapshot_trims_prefix_children() {
        let root = p(1);
        let hub = p(2);
        let inst = p(3);

        let parents = vec![n(root, None), n(hub, Some(root)), n(inst, Some(hub))];
        let mut children_map = HashMap::new();
        children_map.insert(root, vec![c(hub)]);
        children_map.insert(hub, vec![c(inst)]);
        children_map.insert(inst, vec![]);

        let snapshot = TopologySnapshotView {
            parents,
            children_map,
        };

        let sliced = slice_snapshot_for_child(hub, &snapshot).unwrap();

        assert!(!sliced.children_map.contains_key(&root));
        assert!(sliced.children_map.contains_key(&hub));
        assert!(sliced.children_map.contains_key(&inst));
    }

    #[test]
    fn next_child_errors_when_missing_self() {
        let root = p(1);
        let hub = p(2);

        let parents = vec![n(root, None), n(hub, Some(root))];
        assert!(next_child_on_path(p(42), &parents).is_err());
    }

    #[test]
    fn next_child_returns_next_hop() {
        let root = p(1);
        let hub = p(2);
        let inst = p(3);

        let parents = vec![n(root, None), n(hub, Some(root)), n(inst, Some(hub))];

        assert_eq!(next_child_on_path(root, &parents).unwrap(), Some(hub));
    }

    #[test]
    fn next_child_returns_none_at_leaf() {
        let inst = p(3);

        let parents = vec![n(inst, Some(p(2)))];

        assert_eq!(next_child_on_path(inst, &parents).unwrap(), None);
    }
}
