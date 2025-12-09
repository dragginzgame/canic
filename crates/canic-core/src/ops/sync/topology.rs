//! Topology synchronization helpers.
//!
//! Captures subsets of the canister graph (subtree and parent chain) and
//! propagates them down the hierarchy so every node maintains an up-to-date
//! view of its surroundings.

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
/// Snapshot describing a canister’s view of the topology
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
pub struct TopologyBundle {
    pub subtree: Vec<CanisterSummary>,
    pub parents: Vec<CanisterSummary>,
}

impl TopologyBundle {
    /// Construct a bundle rooted at the actual root canister.
    pub fn root() -> Result<Self, Error> {
        let root = SubnetCanisterRegistryOps::get_type(&CanisterRole::ROOT)
            .ok_or(SyncOpsError::RootNotFound)?;

        Ok(Self {
            subtree: SubnetCanisterRegistryOps::subtree(root.pid), // subtree rooted at the actual root PID
            parents: vec![root.into()],
        })
    }

    /// Build a new bundle for a given child, rooted at `child_pid`.
    #[must_use]
    pub fn for_child(
        parent_pid: Principal,
        child_pid: Principal,
        subtree: &[CanisterSummary],
        base: &Self,
    ) -> Self {
        let index = SubtreeIndex::new(subtree);
        Self::for_child_indexed(parent_pid, child_pid, base, &index)
    }

    #[must_use]
    pub fn for_child_indexed(
        parent_pid: Principal,
        child_pid: Principal,
        base: &Self,
        index: &SubtreeIndex,
    ) -> Self {
        let child_subtree = collect_child_subtree(child_pid, index);

        // Parents = whatever base had, plus parent
        let mut new_parents = base.parents.clone();

        if let Some(parent_entry) = index.by_pid.get(&parent_pid).cloned() {
            new_parents.push(parent_entry);
        }

        Self {
            subtree: child_subtree,
            parents: new_parents,
        }
    }

    /// Simple debug string for logging
    #[must_use]
    pub fn debug(&self) -> String {
        format!(
            "subtree:{} parents:{}",
            self.subtree.len(),
            self.parents.len(),
        )
    }
}

/// Cascade from root: build fresh bundles per direct child from the registry.
pub async fn root_cascade_topology() -> Result<(), Error> {
    OpsError::require_root()?;

    let root_pid = canister_self();
    let bundle = TopologyBundle::root()?;
    let index = SubtreeIndex::new(&bundle.subtree);

    let children = SubnetCanisterRegistryOps::children(root_pid);
    let child_count = children.len();
    if child_count > 20 {
        log!(
            Topic::Sync,
            Warn,
            "💦 sync.topology: large root cascade to {child_count} children"
        );
    }

    let mut failures = 0;
    for child in children {
        let child_bundle = TopologyBundle::for_child_indexed(root_pid, child.pid, &bundle, &index);
        if let Err(err) = send_bundle(&child.pid, &child_bundle).await {
            failures += 1;

            log!(
                Topic::Sync,
                Warn,
                "💦 sync.topology: failed to cascade to {}: {}",
                child.pid,
                err
            );
        }
    }

    if failures > 0 {
        log!(
            Topic::Sync,
            Warn,
            "💦 sync.topology: {failures} child cascade(s) failed; continuing"
        );
    }

    Ok(())
}

/// Cascade only the branch that contains `target_pid` (best effort; falls back to full cascade).
pub async fn root_cascade_topology_for_pid(target_pid: Principal) -> Result<(), Error> {
    OpsError::require_root()?;

    let root_pid = canister_self();
    let bundle = TopologyBundle::root()?;
    let index = SubtreeIndex::new(&bundle.subtree);

    let path_up = collect_branch_path(target_pid, &index, root_pid);
    if path_up.is_empty() {
        return root_cascade_topology().await;
    }

    let mut path_down = path_up.clone();
    path_down.reverse(); // root_child -> ... -> target

    let root_child = *path_down.first().unwrap();
    let depth = path_down.len();
    if depth > 20 {
        log!(
            Topic::Sync,
            Warn,
            "💦 sync.topology (targeted): large branch depth {depth}"
        );
    }
    let path_str = path_down
        .iter()
        .map(Principal::to_text)
        .collect::<Vec<_>>()
        .join("→");

    log!(
        Topic::Sync,
        Info,
        "🔀 sync.topology (targeted): root_child={root_child} depth={depth} path=[{path_str}]"
    );

    let mut parent_pid = root_pid;
    let mut parent_bundle = bundle;

    for pid in path_down {
        let child_bundle =
            TopologyBundle::for_child_indexed(parent_pid, pid, &parent_bundle, &index);

        if let Err(err) = send_bundle_with_retry(&pid, &child_bundle).await {
            log!(
                Topic::Sync,
                Warn,
                "💦 sync.topology (targeted): fallback to full cascade after error: {err}"
            );
            return root_cascade_topology().await;
        }

        parent_pid = pid;
        parent_bundle = child_bundle;
    }

    Ok(())
}

/// Cascade from a child: trim bundle to the child’s subtree and forward.
pub async fn nonroot_cascade_topology(bundle: &TopologyBundle) -> Result<(), Error> {
    OpsError::deny_root()?;

    // save local topology
    save_topology(bundle)?;

    // Direct children of self (freshly imported during save_state)
    let self_pid = canister_self();
    let index = SubtreeIndex::new(&bundle.subtree);
    let mut failures = 0;
    let children = SubnetCanisterChildrenOps::export();
    let child_count = children.len();
    if child_count > 20 {
        log!(
            Topic::Sync,
            Warn,
            "💦 sync.topology: large nonroot cascade to {child_count} children"
        );
    }

    for child in children {
        let child_bundle = TopologyBundle::for_child_indexed(self_pid, child.pid, bundle, &index);

        if let Err(err) = send_bundle(&child.pid, &child_bundle).await {
            failures += 1;

            log!(
                Topic::Sync,
                Warn,
                "💦 sync.topology: failed to cascade to {}: {}",
                child.pid,
                err
            );
        }
    }

    if failures > 0 {
        log!(
            Topic::Sync,
            Warn,
            "💦 sync.topology: {failures} child cascade(s) failed; continuing"
        );
    }

    Ok(())
}

/// private function to save local state
fn save_topology(bundle: &TopologyBundle) -> Result<(), Error> {
    OpsError::deny_root()?;

    // subnet canister children
    let self_pid = canister_self();
    let direct_children: Vec<_> = bundle
        .subtree
        .iter()
        .filter(|entry| entry.parent_pid == Some(self_pid))
        .cloned()
        .collect();
    SubnetCanisterChildrenOps::import(direct_children);

    Ok(())
}

/// Low-level bundle sender used by cascade helpers.
async fn send_bundle(pid: &Principal, bundle: &TopologyBundle) -> Result<(), Error> {
    // let debug = bundle.debug();
    //   log!(Topic::Sync, Info, "💦 sync.topology: [{debug}] -> {pid}");

    call_and_decode::<Result<(), Error>>(*pid, "canic_sync_topology", bundle).await?
}

///
/// SubtreeIndex
///

pub struct SubtreeIndex {
    by_pid: HashMap<Principal, CanisterSummary>,
    children_by_parent: HashMap<Principal, Vec<Principal>>,
}

impl SubtreeIndex {
    fn new(subtree: &[CanisterSummary]) -> Self {
        let mut by_pid = HashMap::new();
        let mut children_by_parent: HashMap<Principal, Vec<Principal>> = HashMap::new();

        for entry in subtree {
            by_pid.insert(entry.pid, entry.clone());

            if let Some(parent) = entry.parent_pid {
                children_by_parent
                    .entry(parent)
                    .or_default()
                    .push(entry.pid);
            }
        }

        Self {
            by_pid,
            children_by_parent,
        }
    }

    fn parent_of(&self, pid: Principal) -> Option<Principal> {
        self.by_pid.get(&pid).and_then(|entry| entry.parent_pid)
    }
}

fn collect_child_subtree(child_pid: Principal, index: &SubtreeIndex) -> Vec<CanisterSummary> {
    let mut result = Vec::new();
    let mut stack = vec![child_pid];

    while let Some(current) = stack.pop() {
        if let Some(entry) = index.by_pid.get(&current) {
            result.push(entry.clone());
        }

        if let Some(children) = index.children_by_parent.get(&current) {
            stack.extend(children.iter().copied());
        }
    }

    result
}

fn collect_branch_path(
    mut target_pid: Principal,
    index: &SubtreeIndex,
    root_pid: Principal,
) -> Vec<Principal> {
    let mut path = vec![target_pid];

    loop {
        let Some(parent) = index.parent_of(target_pid) else {
            return vec![];
        };

        if parent == root_pid {
            path.push(parent);
            break;
        }

        path.push(parent);
        target_pid = parent;
    }

    path
}

async fn send_bundle_with_retry(pid: &Principal, bundle: &TopologyBundle) -> Result<(), Error> {
    match send_bundle(pid, bundle).await {
        Ok(()) => Ok(()),
        Err(first_err) => {
            if let Err(second_err) = send_bundle(pid, bundle).await {
                Err(second_err)
            } else {
                log!(
                    Topic::Sync,
                    Warn,
                    "🔁 sync.topology (targeted): retry to {pid} succeeded after: {first_err}"
                );
                Ok(())
            }
        }
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::CanisterRole;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn summary(pid: Principal, parent_pid: Option<Principal>) -> CanisterSummary {
        CanisterSummary {
            pid,
            ty: CanisterRole::new("test"),
            parent_pid,
        }
    }

    #[test]
    fn build_child_subtree_returns_only_descendants() {
        let root = p(1);
        let alpha = p(2);
        let beta = p(3);
        let alpha_a = p(4);
        let alpha_b = p(5);
        let alpha_b_child = p(6);

        let subtree = vec![
            summary(root, None),
            summary(alpha, Some(root)),
            summary(beta, Some(root)),
            summary(alpha_a, Some(alpha)),
            summary(alpha_b, Some(alpha)),
            summary(alpha_b_child, Some(alpha_b)),
        ];

        let index = SubtreeIndex::new(&subtree);
        let mut child_subtree = collect_child_subtree(alpha, &index);
        child_subtree.sort_by(|a, b| a.pid.as_slice().cmp(b.pid.as_slice()));

        let expected: Vec<Principal> = vec![alpha, alpha_a, alpha_b, alpha_b_child];
        let actual: Vec<Principal> = child_subtree.into_iter().map(|e| e.pid).collect();

        assert_eq!(expected, actual);
    }

    #[test]
    fn collect_branch_path_includes_root_child() {
        let root = p(1);
        let hub = p(2);
        let instance = p(3);
        let ledger = p(4);

        let subtree = vec![
            summary(root, None),
            summary(hub, Some(root)),
            summary(instance, Some(hub)),
            summary(ledger, Some(instance)),
        ];

        let index = SubtreeIndex::new(&subtree);
        let path = collect_branch_path(ledger, &index, root);

        // path is [target, parent, ..., root_child]
        assert_eq!(path, vec![ledger, instance, hub, root]);
    }
}
