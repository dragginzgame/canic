use crate::{
    Error, ThisError,
    memory::{
        AppState, AppStateData, CanisterEntry, CanisterState,
        subnet::{SubnetChildren, SubnetDirectory, SubnetParents, SubnetRegistry},
    },
    ops::OpsError,
    ops::prelude::*,
};

///
/// SyncError
///

#[derive(Debug, ThisError)]
pub enum SyncError {
    #[error("canister not found")]
    CanisterNotFound(Principal),

    #[error("cannot cascade without topology")]
    MissingTopology,
}

///
/// AppStateBundle
///

#[derive(CandidType, Copy, Clone, Debug, Default, Deserialize)]
pub struct AppStateBundle {
    pub app_state: AppStateData,
}

impl AppStateBundle {
    #[must_use]
    pub fn root() -> Self {
        Self {
            app_state: AppState::export(),
        }
    }
}

///
/// DirectoryBundle
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
pub struct DirectoryBundle {
    pub subnet_directory: Vec<CanisterEntry>,
}

impl DirectoryBundle {
    #[must_use]
    pub fn root() -> Self {
        Self {
            subnet_directory: SubnetRegistry::subnet_directory(),
        }
    }
}

///
/// TopologyBundle
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
pub struct TopologyBundle {
    pub descendants: Vec<CanisterEntry>,
    pub parents: Vec<CanisterEntry>,
}

impl TopologyBundle {
    pub fn root() -> Result<Self, Error> {
        let root_parent = CanisterState::try_get_entry()?;

        Ok(Self {
            descendants: SubnetRegistry::export(), // entire tree
            parents: vec![root_parent],
        })
    }
}

///
/// SyncBundle
///

#[derive(CandidType, Debug, Default, Deserialize)]
pub struct SyncBundle {
    app_state: Option<AppStateBundle>,
    directory: Option<DirectoryBundle>,
    topology: Option<TopologyBundle>,
}

impl SyncBundle {
    pub fn root() -> Result<Self, Error> {
        OpsError::require_root()?;

        Ok(Self {
            app_state: Some(AppStateBundle::root()),
            directory: Some(DirectoryBundle::root()),
            topology: Some(TopologyBundle::root()?),
        })
    }

    pub fn with_app_state() -> Result<Self, Error> {
        OpsError::require_root()?;

        Ok(Self {
            app_state: Some(AppStateBundle::root()),
            ..Default::default()
        })
    }

    /// Build a new bundle for a given child, rooted at `child_pid`.
    #[must_use]
    pub fn for_child(
        parent_pid: Principal,
        child_pid: Principal,
        all_descendants: &[CanisterEntry],
        base: &Self,
    ) -> Self {
        // Trim descendants to child's subtree
        let child_descendants: Vec<_> = all_descendants
            .iter()
            .filter(|e| is_in_subtree(child_pid, e, all_descendants))
            .cloned()
            .collect();

        // Start with whatever ancestors the parent already had
        let mut new_parents = base
            .topology
            .as_ref()
            .map(|t| t.parents.clone())
            .unwrap_or_default();

        // Add the parent itself into the lineage
        if let Some(parent_entry) = all_descendants
            .iter()
            .find(|e| e.pid == parent_pid)
            .cloned()
        {
            new_parents.push(parent_entry);
        }

        Self {
            app_state: base.app_state,
            directory: base.directory.clone(),
            topology: Some(TopologyBundle {
                descendants: child_descendants,
                parents: new_parents,
            }),
        }
    }
}

/// Cascade from root: build fresh bundles per direct child from the registry.
pub async fn root_cascade() -> Result<(), Error> {
    OpsError::require_root()?; // safeguard

    let root_pid = canister_self();
    let app_state = AppStateBundle::root();
    let directory = DirectoryBundle::root();
    let all_descendants = SubnetRegistry::export();

    for child in all_descendants
        .iter()
        .filter(|e| e.parent_pid == Some(root_pid))
    {
        // Build child-specific bundle
        let child_bundle = SyncBundle {
            app_state: Some(app_state),
            directory: Some(directory.clone()),
            topology: Some(TopologyBundle {
                descendants: SubnetRegistry::descendants(child.pid),
                parents: vec![SubnetRegistry::try_get(root_pid)?],
            }),
        };

        send_bundle(&child.pid, &child_bundle).await?;
    }

    Ok(())
}

/// Cascade from a child: trim bundle to subtree and forward.
pub async fn cascade_children(bundle: &SyncBundle) -> Result<(), Error> {
    OpsError::deny_root()?; // safeguard
    let self_pid = canister_self();

    let topo = bundle
        .topology
        .as_ref()
        .ok_or_else(|| OpsError::from(SyncError::MissingTopology))?;

    // derive direct children from bundle.descendants
    let direct_children: Vec<_> = topo
        .descendants
        .iter()
        .filter(|e| e.parent_pid == Some(self_pid))
        .cloned()
        .collect();

    for child in direct_children {
        let child_bundle = SyncBundle::for_child(self_pid, child.pid, &topo.descendants, bundle);
        send_bundle(&child.pid, &child_bundle).await?;
    }

    Ok(())
}

/// Save state locally on a child canister.
pub fn save_state(bundle: &SyncBundle) -> Result<(), Error> {
    OpsError::deny_root()?;
    let self_pid = canister_self();

    if let Some(app) = &bundle.app_state {
        AppState::import(app.app_state);
        log!(Log::Info, "sync.save_state: app_state imported");
    }

    if let Some(dir) = &bundle.directory {
        let n = dir.subnet_directory.len();
        SubnetDirectory::import(dir.subnet_directory.clone());
        log!(
            Log::Info,
            "sync.save_state: directory imported ({n} entries)",
        );
    }

    if let Some(top) = &bundle.topology {
        log!(
            Log::Info,
            "sync.save_state: topology received (descendants={}, parents={})",
            top.descendants.len(),
            top.parents.len()
        );

        let self_entry = top
            .descendants
            .iter()
            .find(|e| e.pid == self_pid)
            .cloned()
            .ok_or_else(|| OpsError::from(SyncError::CanisterNotFound(self_pid)))?;

        CanisterState::set_entry(self_entry);

        SubnetParents::import(top.parents.clone());
        log!(
            Log::Info,
            "sync.save_state: parents imported ({} entries)",
            top.parents.len()
        );

        let direct_children: Vec<_> = top
            .descendants
            .iter()
            .filter(|e| e.parent_pid == Some(self_pid))
            .cloned()
            .collect();

        let m = direct_children.len();
        SubnetChildren::import(direct_children);
        log!(
            Log::Info,
            "sync.save_state: children imported ({m} entries)",
        );
    }

    Ok(())
}

/// Check if `entry` is part of the subtree rooted at `root_pid`.
fn is_in_subtree(root_pid: Principal, entry: &CanisterEntry, all: &[CanisterEntry]) -> bool {
    let mut current = entry.parent_pid;

    while let Some(pid) = current {
        if pid == root_pid {
            return true;
        }
        // climb up the chain
        current = all.iter().find(|e| e.pid == pid).and_then(|e| e.parent_pid);
    }

    false
}

/// Low-level bundle sender.
async fn send_bundle(pid: &Principal, bundle: &SyncBundle) -> Result<(), Error> {
    log!(Log::Info, "💦 state.cascade: -> {pid}");

    call_and_decode::<Result<(), Error>>(*pid, "icu_sync_cascade", bundle).await?
}
