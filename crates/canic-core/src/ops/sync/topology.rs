//! Topology synchronization helpers.
//!
//! Extracts relevant topology subsets and propagates them so each canister
//! maintains an up-to-date view of its children and parent chain.

use super::warn_if_large;
use crate::{
    Error,
    log::Topic,
    model::memory::CanisterSummary,
    ops::{
        OpsError,
        model::memory::topology::subnet::{SubnetCanisterChildrenOps, SubnetCanisterRegistryOps},
        prelude::*,
        sync::SyncOpsError,
    },
};
use std::collections::HashMap;

///
/// TopologyBundle
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
pub struct TopologyBundle {
    pub parents: Vec<CanisterSummary>,
    pub children_map: HashMap<Principal, Vec<CanisterSummary>>,
}

impl TopologyBundle {
    pub fn for_target(target_pid: Principal) -> Result<Self, Error> {
        let parents = parent_chain(target_pid)?;

        let mut children_map = HashMap::new();

        // Add children of every ancestor in the chain (root → ... → target)
        for parent in &parents {
            let children = SubnetCanisterRegistryOps::children(parent.pid);
            children_map.insert(parent.pid, children);
        }

        Ok(Self {
            parents,
            children_map,
        })
    }
}

//
// ===========================================================================
//  ROOT CASCADES
// ===========================================================================
//

pub(crate) async fn root_cascade_topology_for_pid(target_pid: Principal) -> Result<(), Error> {
    OpsError::require_root()?;

    let bundle = TopologyBundle::for_target(target_pid)?;
    let root_pid = canister_self();
    let Some(first_child) = next_child_on_path(root_pid, &bundle.parents) else {
        log!(
            Topic::Sync,
            Warn,
            "sync.topology: no branch path to {target_pid}, skipping targeted cascade"
        );

        return Ok(());
    };

    let child_bundle = slice_bundle_for_child(first_child, &bundle);
    if let Err(err) = send_bundle(&first_child, &child_bundle).await {
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

pub async fn nonroot_cascade_topology(bundle: &TopologyBundle) -> Result<(), Error> {
    OpsError::deny_root()?;
    let self_pid = canister_self();

    let children = bundle
        .children_map
        .get(&self_pid)
        .cloned()
        .unwrap_or_default();
    warn_if_large("nonroot fanout", children.len());
    SubnetCanisterChildrenOps::import(children);

    if let Some(next_pid) = next_child_on_path(self_pid, &bundle.parents) {
        let next_bundle = slice_bundle_for_child(next_pid, bundle);
        send_bundle(&next_pid, &next_bundle).await?;
    }

    Ok(())
}

//
// ===========================================================================
//  HELPERS
// ===========================================================================
//

fn parent_chain(mut pid: Principal) -> Result<Vec<CanisterSummary>, Error> {
    let mut chain = Vec::new();

    loop {
        let Some(entry) = SubnetCanisterRegistryOps::get(pid) else {
            return Err(SyncOpsError::CanisterNotFound(pid).into());
        };

        chain.push(entry.clone().into());

        let Some(parent) = entry.parent_pid else {
            break;
        };
        pid = parent;
    }

    chain.reverse();
    Ok(chain)
}

async fn send_bundle(pid: &Principal, bundle: &TopologyBundle) -> Result<(), Error> {
    call_and_decode::<Result<(), Error>>(*pid, "canic_sync_topology", bundle).await?
}

//
// ===========================================================================
//  PATH HELPERS
// ===========================================================================
//

fn next_child_on_path(self_pid: Principal, parents: &[CanisterSummary]) -> Option<Principal> {
    let idx = parents.iter().position(|p| p.pid == self_pid)?;
    parents.get(idx + 1).map(|p| p.pid)
}

fn slice_bundle_for_child(next_pid: Principal, bundle: &TopologyBundle) -> TopologyBundle {
    // Slice parents chain so we start at the next hop
    let mut sliced_parents = Vec::new();
    let mut include = false;

    for parent in &bundle.parents {
        if parent.pid == next_pid {
            include = true;
        }
        if include {
            sliced_parents.push(parent.clone());
        }
    }

    // Slice children_map so it includes only nodes in the sliced chain
    let mut sliced_children_map = HashMap::new();
    for parent in &sliced_parents {
        let children = bundle
            .children_map
            .get(&parent.pid)
            .cloned()
            .unwrap_or_default();
        sliced_children_map.insert(parent.pid, children);
    }

    TopologyBundle {
        parents: sliced_parents,
        children_map: sliced_children_map,
    }
}

//
// ===========================================================================
//  TESTS
// ===========================================================================
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::CanisterRole;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn s(pid: Principal, parent_pid: Option<Principal>) -> CanisterSummary {
        CanisterSummary {
            pid,
            ty: CanisterRole::new("test"),
            parent_pid,
        }
    }

    #[test]
    fn next_child_from_chain() {
        let root = p(1);
        let hub = p(2);
        let inst = p(3);
        let ledg = p(4);

        let parents = vec![
            s(root, None),
            s(hub, Some(root)),
            s(inst, Some(hub)),
            s(ledg, Some(inst)),
        ];

        assert_eq!(next_child_on_path(root, &parents), Some(hub));
        assert_eq!(next_child_on_path(hub, &parents), Some(inst));
        assert_eq!(next_child_on_path(ledg, &parents), None);
    }

    #[test]
    fn slice_bundle_trims_prefix_children() {
        let root = p(1);
        let hub = p(2);
        let inst = p(3);

        let parents = vec![s(root, None), s(hub, Some(root)), s(inst, Some(hub))];
        let mut children_map = HashMap::new();
        children_map.insert(root, vec![s(hub, Some(root))]);
        children_map.insert(hub, vec![s(inst, Some(hub))]);
        children_map.insert(inst, vec![]);

        let bundle = TopologyBundle {
            parents,
            children_map,
        };

        let sliced = slice_bundle_for_child(hub, &bundle);

        assert!(!sliced.children_map.contains_key(&root));
        assert!(sliced.children_map.contains_key(&hub));
        assert!(sliced.children_map.contains_key(&inst));
    }
}
