//! Topology synchronization helpers.
//!
//! Captures subsets of the canister graph (subtree and parent chain) and
//! propagates them down the hierarchy so every node maintains an up-to-date
//! view of its surroundings.

use crate::{
    Error,
    log::Topic,
    model::memory::{
        CanisterSummary,
        topology::{SubnetCanisterChildren, SubnetCanisterRegistry},
    },
    ops::{OpsError, prelude::*},
};

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
        let root = SubnetCanisterRegistry::try_get_type(&CanisterType::ROOT)?;

        Ok(Self {
            subtree: SubnetCanisterRegistry::subtree(root.pid), // subtree rooted at the actual root PID
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
        // Trim subtree to child’s subtree
        let child_subtree: Vec<_> = subtree
            .iter()
            .filter(|e| SubnetCanisterRegistry::is_in_subtree(child_pid, e, subtree))
            .cloned()
            .collect();

        // Parents = whatever base had, plus parent
        let mut new_parents = base.parents.clone();

        if let Some(parent_entry) = subtree.iter().find(|e| e.pid == parent_pid).cloned() {
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

    for child in SubnetCanisterRegistry::children(root_pid) {
        let child_bundle = TopologyBundle::for_child(root_pid, child.pid, &bundle.subtree, &bundle);
        send_bundle(&child.pid, &child_bundle).await?;
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
    for child in SubnetCanisterChildren::export() {
        let child_bundle = TopologyBundle::for_child(self_pid, child.pid, &bundle.subtree, bundle);
        send_bundle(&child.pid, &child_bundle).await?;
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
    SubnetCanisterChildren::import(direct_children);

    Ok(())
}

/// Low-level bundle sender used by cascade helpers.
async fn send_bundle(pid: &Principal, bundle: &TopologyBundle) -> Result<(), Error> {
    let debug = bundle.debug();
    log!(
        Topic::CanisterState,
        Info,
        "💦 sync.topology: [{debug}] -> {pid}"
    );

    call_and_decode::<Result<(), Error>>(*pid, "canic_sync_topology", bundle).await?
}
