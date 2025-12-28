//!
//! Snapshot assembly helpers.
//!
//! This module:
//! - assembles snapshot DTOs from authoritative state via ops
//! - exposes builder-style APIs for snapshot construction
//! - keeps DTOs data-only by placing helper logic here
//!

use crate::{
    Error,
    dto::{
        directory::{AppDirectoryView, SubnetDirectoryView},
        snapshot::{StateSnapshotView, TopologyNodeView, TopologySnapshotView},
        state::{AppModeView, AppStateView, SubnetStateView},
    },
    ids::CanisterRole,
    model::memory::{
        CanisterEntry, CanisterSummary,
        state::{AppMode, AppStateData, SubnetStateData},
    },
    ops::{
        adapter::{
            directory::{
                app_directory_from_view, app_directory_to_view, subnet_directory_from_view,
                subnet_directory_to_view,
            },
            state::app_mode_into_view,
        },
        storage::{
            directory::{AppDirectoryOps, SubnetDirectoryOps},
            registry::SubnetRegistryOps,
            state::{AppStateOps, SubnetStateOps},
        },
    },
    workflow::cascade::CascadeError,
};
use candid::Principal;
use std::collections::{HashMap, HashSet};

///
/// StateSnapshotBuilder
///
/// Assembles `StateSnapshotView` DTOs from authoritative state.
/// This is workflow code (not DTO, not ops).
///

#[derive(Default)]
pub struct StateSnapshotBuilder {
    snapshot: StateSnapshotView,
}

impl StateSnapshotBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            snapshot: StateSnapshotView::default(),
        }
    }

    /// Construct a snapshot containing the full root state.
    #[must_use]
    pub fn root() -> Self {
        Self {
            snapshot: StateSnapshotView {
                app_state: Some(app_state_view_from_data(AppStateOps::export())),
                subnet_state: Some(subnet_state_view_from_data(SubnetStateOps::export())),
                app_directory: Some(app_directory_to_view(AppDirectoryOps::export())),
                subnet_directory: Some(subnet_directory_to_view(SubnetDirectoryOps::export())),
            },
        }
    }

    #[must_use]
    pub fn with_app_state(mut self) -> Self {
        self.snapshot.app_state = Some(app_state_view_from_data(AppStateOps::export()));
        self
    }

    #[must_use]
    pub fn with_subnet_state(mut self) -> Self {
        self.snapshot.subnet_state = Some(subnet_state_view_from_data(SubnetStateOps::export()));
        self
    }

    #[must_use]
    pub fn with_app_directory(mut self) -> Self {
        self.snapshot.app_directory = Some(app_directory_to_view(AppDirectoryOps::export()));
        self
    }

    #[must_use]
    pub fn with_subnet_directory(mut self) -> Self {
        self.snapshot.subnet_directory =
            Some(subnet_directory_to_view(SubnetDirectoryOps::export()));
        self
    }

    #[must_use]
    pub const fn with_app_state_view(mut self, view: AppStateView) -> Self {
        self.snapshot.app_state = Some(view);
        self
    }

    #[must_use]
    pub const fn with_subnet_state_view(mut self, view: SubnetStateView) -> Self {
        self.snapshot.subnet_state = Some(view);
        self
    }

    #[must_use]
    pub fn with_app_directory_view(mut self, view: AppDirectoryView) -> Self {
        self.snapshot.app_directory = Some(view);
        self
    }

    #[must_use]
    pub fn with_subnet_directory_view(mut self, view: SubnetDirectoryView) -> Self {
        self.snapshot.subnet_directory = Some(view);
        self
    }

    #[must_use]
    pub fn build(self) -> StateSnapshotView {
        self.snapshot
    }
}

///
/// TopologySnapshotBuilder
///
/// Workflow helper for assembling topology snapshots.
///
pub struct TopologySnapshotBuilder {
    snapshot: TopologySnapshotView,
}

impl TopologySnapshotBuilder {
    pub fn for_target(target_pid: Principal) -> Result<Self, Error> {
        let parents = parent_chain(target_pid)?;
        let mut children_map = HashMap::new();

        for parent in &parents {
            let children = SubnetRegistryOps::children(parent.pid)
                .into_iter()
                .map(|(pid, summary)| topology_node_from_summary(pid, &summary))
                .collect();
            children_map.insert(parent.pid, children);
        }

        Ok(Self {
            snapshot: TopologySnapshotView {
                parents,
                children_map,
            },
        })
    }

    #[must_use]
    pub fn build(self) -> TopologySnapshotView {
        self.snapshot
    }
}

// -----------------------------------------------------------------------------
// Snapshot helpers (workflow-owned)
// -----------------------------------------------------------------------------

#[must_use]
pub(crate) const fn state_snapshot_is_empty(snapshot: &StateSnapshotView) -> bool {
    snapshot.app_state.is_none()
        && snapshot.subnet_state.is_none()
        && snapshot.app_directory.is_none()
        && snapshot.subnet_directory.is_none()
}

#[must_use]
pub(crate) fn state_snapshot_debug(snapshot: &StateSnapshotView) -> String {
    const fn fmt(present: bool, code: &str) -> &str {
        if present { code } else { ".." }
    }

    format!(
        "[{} {} {} {}]",
        fmt(snapshot.app_state.is_some(), "as"),
        fmt(snapshot.subnet_state.is_some(), "ss"),
        fmt(snapshot.app_directory.is_some(), "ad"),
        fmt(snapshot.subnet_directory.is_some(), "sd"),
    )
}

#[must_use]
pub(crate) const fn app_state_view_from_data(data: AppStateData) -> AppStateView {
    AppStateView {
        mode: app_mode_into_view(data.mode),
    }
}

#[must_use]
pub(crate) const fn subnet_state_view_from_data(_data: SubnetStateData) -> SubnetStateView {
    SubnetStateView {}
}

#[must_use]
pub(crate) const fn app_state_data_from_view(view: AppStateView) -> AppStateData {
    AppStateData {
        mode: app_mode_from_view(view.mode),
    }
}

#[must_use]
pub(crate) const fn subnet_state_data_from_view(_view: SubnetStateView) -> SubnetStateData {
    SubnetStateData {}
}

#[must_use]
pub(crate) fn app_directory_data_from_view(
    view: AppDirectoryView,
) -> Vec<(CanisterRole, Principal)> {
    app_directory_from_view(view)
}

#[must_use]
pub(crate) fn subnet_directory_data_from_view(
    view: SubnetDirectoryView,
) -> Vec<(CanisterRole, Principal)> {
    subnet_directory_from_view(view)
}

const fn app_mode_from_view(mode: AppModeView) -> AppMode {
    match mode {
        AppModeView::Enabled => AppMode::Enabled,
        AppModeView::Readonly => AppMode::Readonly,
        AppModeView::Disabled => AppMode::Disabled,
    }
}

fn topology_node_from_entry(pid: Principal, entry: &CanisterEntry) -> TopologyNodeView {
    TopologyNodeView {
        pid,
        role: entry.role.clone(),
        parent_pid: entry.parent_pid,
    }
}

fn topology_node_from_summary(pid: Principal, summary: &CanisterSummary) -> TopologyNodeView {
    TopologyNodeView {
        pid,
        role: summary.role.clone(),
        parent_pid: summary.parent_pid,
    }
}

fn parent_chain(mut pid: Principal) -> Result<Vec<TopologyNodeView>, Error> {
    let registry_len = SubnetRegistryOps::export().len();
    let mut chain = Vec::new();
    let mut seen: HashSet<Principal> = HashSet::new();

    loop {
        if !seen.insert(pid) {
            return Err(CascadeError::ParentChainCycle(pid).into());
        }

        let Some(entry) = SubnetRegistryOps::get(pid) else {
            return Err(CascadeError::CanisterNotFound(pid).into());
        };

        if seen.len() > registry_len {
            return Err(CascadeError::ParentChainTooLong(seen.len()).into());
        }

        chain.push(topology_node_from_entry(pid, &entry));

        let Some(parent) = entry.parent_pid else {
            if entry.role != CanisterRole::ROOT {
                return Err(CascadeError::ParentChainNotRootTerminated(pid).into());
            }
            break;
        };
        pid = parent;
    }

    chain.reverse();

    Ok(chain)
}
