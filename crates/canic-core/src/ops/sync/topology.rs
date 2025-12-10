//! Topology synchronization helpers.
//!
//! Extracts relevant topology subsets and propagates them so each canister
//! maintains an up-to-date view of its children and parent chain.

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

//
// ===========================================================================
//  BUNDLES
// ===========================================================================
//

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
pub struct TopologyBundle {
    pub subtree: Vec<CanisterSummary>,
    pub parents: Vec<CanisterSummary>,
}

impl TopologyBundle {
    pub fn root() -> Result<Self, Error> {
        let root = SubnetCanisterRegistryOps::get_type(&CanisterRole::ROOT)
            .ok_or(SyncOpsError::RootNotFound)?;
        Ok(Self {
            subtree: SubnetCanisterRegistryOps::subtree(root.pid),
            parents: vec![root.into()],
        })
    }

    #[must_use]
    pub fn for_child(
        parent_pid: Principal,
        child_pid: Principal,
        base: &Self,
        index: &SubtreeIndex,
    ) -> Self {
        let mut parents = base.parents.clone();
        if let Some(parent) = index.by_pid.get(&parent_pid) {
            parents.push(parent.clone());
        }
        Self {
            subtree: collect_child_subtree(child_pid, index),
            parents,
        }
    }
}

//
// ===========================================================================
//  ROOT CASCADES
// ===========================================================================
//

pub async fn root_cascade_topology() -> Result<(), Error> {
    OpsError::require_root()?;

    let root_pid = canister_self();
    let bundle = TopologyBundle::root()?;
    let index = SubtreeIndex::new(&bundle.subtree);

    let children = SubnetCanisterRegistryOps::children(root_pid);
    warn_large(children.len(), "root");

    cascade_children(root_pid, &bundle, &index, children).await
}

pub async fn root_cascade_topology_for_pid(target_pid: Principal) -> Result<(), Error> {
    OpsError::require_root()?;

    let root_pid = canister_self();
    let bundle = TopologyBundle::root()?;
    let index = SubtreeIndex::new(&bundle.subtree);

    // Upward path: [target, parent, ..., root_child, root]
    let path = collect_branch_path(target_pid, &index, root_pid);
    if path.is_empty() {
        log!(
            Topic::Sync,
            Warn,
            "sync.topology: no branch path to {target_pid}, skipping targeted cascade"
        );
        return Ok(());
    }

    // Descending: [root, root_child, ..., target]
    let mut down: Vec<_> = path.into_iter().rev().collect();

    // Must begin with root
    if down.first().copied() != Some(root_pid) {
        log!(
            Topic::Sync,
            Warn,
            "sync.topology: branch path for {target_pid} does not start at root, skipping targeted cascade"
        );
        return Ok(());
    }

    // Skip root itself
    down.remove(0);
    if down.is_empty() {
        return Ok(());
    }

    // Hand off to the first child; that branch will cascade onward via non-root logic.
    let first_child = down.remove(0);
    let child_bundle = TopologyBundle::for_child(root_pid, first_child, &bundle, &index);
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
            "sync.topology: delegated targeted cascade to {first_child} (depth={})",
            down.len() + 1
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
    save_topology(bundle);

    let self_pid = canister_self();
    let index = SubtreeIndex::new(&bundle.subtree);
    let children = SubnetCanisterChildrenOps::export();
    warn_large(children.len(), "nonroot");

    cascade_children(self_pid, bundle, &index, children).await
}

//
// ===========================================================================
//  HELPERS
// ===========================================================================
//

async fn cascade_children(
    parent_pid: Principal,
    bundle: &TopologyBundle,
    index: &SubtreeIndex,
    children: Vec<CanisterSummary>,
) -> Result<(), Error> {
    let mut failures = 0;

    for child in children {
        let child_bundle = TopologyBundle::for_child(parent_pid, child.pid, bundle, index);
        if let Err(err) = send_bundle(&child.pid, &child_bundle).await {
            failures += 1;
            log!(
                Topic::Sync,
                Warn,
                "sync.topology: failed cascade to {}: {}",
                child.pid,
                err
            );
        }
    }

    if failures > 0 {
        log!(
            Topic::Sync,
            Warn,
            "sync.topology: {failures} cascade(s) failed"
        );
    }

    Ok(())
}

fn warn_large(n: usize, label: &str) {
    if n > 10 {
        log!(
            Topic::Sync,
            Warn,
            "sync.topology: large {label} fanout: {n}"
        );
    }
}

fn save_topology(bundle: &TopologyBundle) {
    let self_pid = canister_self();
    let direct: Vec<_> = bundle
        .subtree
        .iter()
        .filter(|e| e.parent_pid == Some(self_pid))
        .cloned()
        .collect();

    SubnetCanisterChildrenOps::import(direct);
}

async fn send_bundle(pid: &Principal, bundle: &TopologyBundle) -> Result<(), Error> {
    call_and_decode::<Result<(), Error>>(*pid, "canic_sync_topology", bundle).await?
}

//
// ===========================================================================
//  INDEX + TRAVERSAL
// ===========================================================================
//

pub struct SubtreeIndex {
    by_pid: HashMap<Principal, CanisterSummary>,
    children_by_parent: HashMap<Principal, Vec<Principal>>,
}

impl SubtreeIndex {
    fn new(subtree: &[CanisterSummary]) -> Self {
        let mut by_pid: HashMap<Principal, CanisterSummary> = HashMap::new();
        let mut children_by_parent: HashMap<Principal, Vec<Principal>> = HashMap::new();

        for entry in subtree {
            by_pid.insert(entry.pid, entry.clone());
            if let Some(p) = entry.parent_pid {
                children_by_parent.entry(p).or_default().push(entry.pid);
            }
        }

        Self {
            by_pid,
            children_by_parent,
        }
    }

    fn parent_of(&self, pid: Principal) -> Option<Principal> {
        self.by_pid.get(&pid).and_then(|e| e.parent_pid)
    }
}

fn collect_child_subtree(root: Principal, index: &SubtreeIndex) -> Vec<CanisterSummary> {
    let mut result = Vec::new();
    let mut stack = vec![root];

    while let Some(pid) = stack.pop() {
        if let Some(entry) = index.by_pid.get(&pid) {
            result.push(entry.clone());
        }
        if let Some(children) = index.children_by_parent.get(&pid) {
            stack.extend(children.iter().copied());
        }
    }

    result
}

fn collect_branch_path(
    mut pid: Principal,
    index: &SubtreeIndex,
    root_pid: Principal,
) -> Vec<Principal> {
    let mut path = vec![pid];
    loop {
        let Some(p) = index.parent_of(pid) else {
            return Vec::new();
        };
        path.push(p);
        if p == root_pid {
            break;
        }
        pid = p;
    }
    path
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
    fn subtree_descendants() {
        let root = p(1);
        let a = p(2);
        let b = p(3);
        let a1 = p(4);
        let a2 = p(5);
        let a2c = p(6);

        let st = vec![
            s(root, None),
            s(a, Some(root)),
            s(b, Some(root)),
            s(a1, Some(a)),
            s(a2, Some(a)),
            s(a2c, Some(a2)),
        ];

        let index = SubtreeIndex::new(&st);
        let mut out = collect_child_subtree(a, &index);
        out.sort_by_key(|e| e.pid.as_slice().to_vec());

        assert_eq!(
            out.into_iter().map(|e| e.pid).collect::<Vec<_>>(),
            vec![a, a1, a2, a2c]
        );
    }

    #[test]
    fn branch_path() {
        let root = p(1);
        let hub = p(2);
        let inst = p(3);
        let ledg = p(4);

        let st = vec![
            s(root, None),
            s(hub, Some(root)),
            s(inst, Some(hub)),
            s(ledg, Some(inst)),
        ];

        let index = SubtreeIndex::new(&st);
        assert_eq!(
            collect_branch_path(ledg, &index, root),
            vec![ledg, inst, hub, root]
        );
    }
}
