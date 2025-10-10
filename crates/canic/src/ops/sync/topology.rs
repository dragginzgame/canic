//! Topology synchronization helpers.
//!
//! Captures subsets of the canister graph (subtree, parents, directory) and
//! propagates them down the hierarchy so every node maintains an up-to-date
//! view of its surroundings.

use crate::{
    Error,
    memory::{
        CanisterSummary,
        state::CanisterState,
        topology::{
            SubnetCanisterChildren, SubnetCanisterDirectory, SubnetCanisterParents,
            SubnetCanisterRegistry,
        },
    },
    ops::{OpsError, prelude::*, sync::SyncError},
};

///
/// TopologyBundle
/// Snapshot describing a canister’s view of the topology
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
pub struct TopologyBundle {
    pub subtree: Vec<CanisterSummary>,
    pub parents: Vec<CanisterSummary>,
    pub directory: Vec<CanisterSummary>,
}

impl TopologyBundle {
    /// Construct a bundle rooted at the actual root canister.
    pub fn root() -> Result<Self, Error> {
        let root_summary = CanisterState::try_get_canister()?;
        let root_pid = root_summary.pid;

        Ok(Self {
            subtree: SubnetCanisterRegistry::subtree(root_pid), // subtree rooted at the actual root PID
            parents: vec![root_summary],
            directory: SubnetCanisterRegistry::directory(),
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
            directory: base.directory.clone(),
        }
    }

    /// Simple debug string for logging
    #[must_use]
    pub fn debug(&self) -> String {
        format!(
            "subtree:{} parents:{} dir:{}",
            self.subtree.len(),
            self.parents.len(),
            self.directory.len()
        )
    }
}

/// Cascade from root: build fresh bundles per direct child from the registry.
pub async fn root_cascade() -> Result<(), Error> {
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
pub async fn cascade_children(bundle: &TopologyBundle) -> Result<(), Error> {
    OpsError::deny_root()?;
    let self_pid = canister_self();

    // Direct children of self (freshly imported during save_state)
    for child in SubnetCanisterChildren::export() {
        let child_bundle = TopologyBundle::for_child(self_pid, child.pid, &bundle.subtree, bundle);
        send_bundle(&child.pid, &child_bundle).await?;
    }

    Ok(())
}

/// Save topology state locally on a child canister.
pub fn save_state(bundle: &TopologyBundle) -> Result<(), Error> {
    OpsError::deny_root()?;

    // canister state
    let self_pid = canister_self();
    let self_entry = bundle
        .subtree
        .iter()
        .find(|e| e.pid == self_pid)
        .cloned()
        .ok_or(SyncError::CanisterNotFound(self_pid))?;
    CanisterState::set_canister(self_entry);

    // subnet canister parents
    SubnetCanisterParents::import(bundle.parents.clone());

    // subnet canister children
    let direct_children: Vec<_> = bundle
        .subtree
        .iter()
        .filter(|entry| entry.parent_pid == Some(self_pid))
        .cloned()
        .collect();
    SubnetCanisterChildren::import(direct_children);

    // subnet canister directory
    SubnetCanisterDirectory::import(bundle.directory.clone());

    Ok(())
}

/// Low-level bundle sender used by cascade helpers.
async fn send_bundle(pid: &Principal, bundle: &TopologyBundle) -> Result<(), Error> {
    let debug = bundle.debug();
    log!(Log::Info, "💦 sync.topology: [{debug}] -> {pid}");

    call_and_decode::<Result<(), Error>>(*pid, "canic_sync_topology", bundle).await?
}
