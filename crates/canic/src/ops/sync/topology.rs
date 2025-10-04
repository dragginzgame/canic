use crate::{
    Error,
    memory::{
        CanisterSummary,
        state::CanisterState,
        topology::{SubnetChildren, SubnetDirectory, SubnetParents, SubnetTopology},
    },
    ops::{OpsError, prelude::*, sync::SyncError},
};

///
/// TopologyBundle
/// any time the subnet topology changes we sync this
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
pub struct TopologyBundle {
    pub subtree: Vec<CanisterSummary>,
    pub parents: Vec<CanisterSummary>,
    pub directory: Vec<CanisterSummary>,
}

impl TopologyBundle {
    pub fn root() -> Result<Self, Error> {
        let root_summary = CanisterState::try_get_canister()?;
        let root_pid = root_summary.pid;

        Ok(Self {
            subtree: SubnetTopology::subtree(root_pid), // subtree rooted at the actual root PID
            parents: vec![root_summary],
            directory: SubnetTopology::directory(),
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
            .filter(|e| SubnetTopology::is_in_subtree(child_pid, e, subtree))
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

    for child in SubnetTopology::children(root_pid) {
        let child_bundle = TopologyBundle::for_child(root_pid, child.pid, &bundle.subtree, &bundle);
        send_bundle(&child.pid, &child_bundle).await?;
    }

    Ok(())
}

/// Cascade from a child: trim bundle to subtree and forward.
pub async fn cascade_children(bundle: &TopologyBundle) -> Result<(), Error> {
    OpsError::deny_root()?;
    let self_pid = canister_self();

    // Direct children of self (freshly imported during save_state)
    for child in SubnetChildren::export() {
        let child_bundle = TopologyBundle::for_child(self_pid, child.pid, &bundle.subtree, bundle);
        send_bundle(&child.pid, &child_bundle).await?;
    }

    Ok(())
}

/// Save state locally on a child canister.
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

    // subnet parents
    SubnetParents::import(bundle.parents.clone());

    // subnet children
    let direct_children: Vec<_> = bundle
        .subtree
        .iter()
        .filter(|entry| entry.parent_pid == Some(self_pid))
        .cloned()
        .collect();
    SubnetChildren::import(direct_children);

    // subnet directory
    SubnetDirectory::import(bundle.directory.clone());

    Ok(())
}

/// Low-level bundle sender.
async fn send_bundle(pid: &Principal, bundle: &TopologyBundle) -> Result<(), Error> {
    let debug = bundle.debug();
    log!(Log::Info, "💦 sync.topology: [{debug}] -> {pid}");

    call_and_decode::<Result<(), Error>>(*pid, "canic_sync_topology", bundle).await?
}
