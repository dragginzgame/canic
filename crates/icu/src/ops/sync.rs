use crate::{
    Error, ThisError,
    memory::{
        CanisterView,
        app::{AppState, AppStateData},
        canister::CanisterState,
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
    pub subnet_directory: Vec<CanisterView>,
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
    pub subtree: Vec<CanisterView>,
    pub parents: Vec<CanisterView>,
}

impl TopologyBundle {
    pub fn root() -> Result<Self, Error> {
        let root_parent = CanisterState::try_get_view()?;

        Ok(Self {
            subtree: SubnetRegistry::export_views(), // entire tree
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
        subtree: &[CanisterView],
        base: &Self,
    ) -> Self {
        // Trim subtree to child’s subtree
        let child_subtree: Vec<_> = subtree
            .iter()
            .filter(|e| SubnetRegistry::is_in_subtree(child_pid, e, subtree))
            .cloned()
            .collect();

        // Parents = whatever base had, plus parent
        let mut new_parents = base
            .topology
            .as_ref()
            .map(|t| t.parents.clone())
            .unwrap_or_default();

        if let Some(parent_entry) = subtree.iter().find(|e| e.pid == parent_pid).cloned() {
            new_parents.push(parent_entry);
        }

        Self {
            app_state: base.app_state,
            directory: base.directory.clone(),
            topology: Some(TopologyBundle {
                subtree: child_subtree,
                parents: new_parents,
            }),
        }
    }

    /// Compact debug string (`adt`) showing which sections are present.
    fn debug(&self) -> String {
        [
            if self.app_state.is_some() { 'a' } else { '.' },
            if self.directory.is_some() { 'd' } else { '.' },
            if self.topology.is_some() { 't' } else { '.' },
        ]
        .iter()
        .collect()
    }
}

/// Cascade from root: build fresh bundles per direct child from the registry.
pub async fn root_cascade() -> Result<(), Error> {
    OpsError::require_root()?;

    let root_pid = canister_self();
    let app_state = AppStateBundle::root();
    let directory = DirectoryBundle::root();
    let all_views = SubnetRegistry::export_views();
    let root_view = CanisterState::try_get_view()?;

    for child in all_views.iter().filter(|e| e.parent_pid == Some(root_pid)) {
        let child_bundle = SyncBundle {
            app_state: Some(app_state),
            directory: Some(directory.clone()),
            topology: Some(TopologyBundle {
                subtree: SubnetRegistry::subtree(child.pid),
                parents: vec![root_view.clone()],
            }),
        };
        send_bundle(&child.pid, &child_bundle).await?;
    }

    Ok(())
}

/// Cascade from a child: trim bundle to subtree and forward.
pub async fn cascade_children(bundle: &SyncBundle) -> Result<(), Error> {
    OpsError::deny_root()?;
    let self_pid = canister_self();

    let topo = bundle
        .topology
        .as_ref()
        .ok_or_else(|| OpsError::from(SyncError::MissingTopology))?;

    // Direct children of self
    let direct_children: Vec<_> = topo
        .subtree
        .iter()
        .filter(|e| e.parent_pid == Some(self_pid))
        .cloned()
        .collect();

    for child in direct_children {
        let child_bundle = SyncBundle::for_child(self_pid, child.pid, &topo.subtree, bundle);
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
    }

    if let Some(dir) = &bundle.directory {
        SubnetDirectory::import(dir.subnet_directory.clone());
    }

    if let Some(top) = &bundle.topology {
        let self_entry = top
            .subtree
            .iter()
            .find(|e| e.pid == self_pid)
            .cloned()
            .ok_or_else(|| OpsError::from(SyncError::CanisterNotFound(self_pid)))?;

        CanisterState::set_view(self_entry);
        SubnetParents::import(top.parents.clone());

        let direct_children: Vec<_> = top
            .subtree
            .iter()
            .filter(|e| e.parent_pid == Some(self_pid))
            .cloned()
            .collect();

        SubnetChildren::import(direct_children);
    }

    Ok(())
}

/// Low-level bundle sender.
async fn send_bundle(pid: &Principal, bundle: &SyncBundle) -> Result<(), Error> {
    let debug = &bundle.debug();
    log!(Log::Info, "💦 state.cascade: [{debug}] -> {pid}");

    call_and_decode::<Result<(), Error>>(*pid, "icu_sync_cascade", bundle).await?
}
