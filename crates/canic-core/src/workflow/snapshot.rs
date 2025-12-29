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
        snapshot::{StateSnapshotView, TopologySnapshotView},
        state::{AppStateView, SubnetStateView},
    },
    ops::storage::{
        directory::{AppDirectoryOps, SubnetDirectoryOps},
        registry::SubnetRegistryOps,
        state::{AppStateOps, SubnetStateOps},
    },
};
use candid::Principal;
use std::collections::HashMap;

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
                app_state: Some(AppStateOps::export_view()),
                subnet_state: Some(SubnetStateOps::export_view()),
                app_directory: Some(AppDirectoryOps::export_view()),
                subnet_directory: Some(SubnetDirectoryOps::export_view()),
            },
        }
    }

    #[must_use]
    pub fn with_app_state(mut self) -> Self {
        self.snapshot.app_state = Some(AppStateOps::export_view());
        self
    }

    #[must_use]
    pub fn with_subnet_state(mut self) -> Self {
        self.snapshot.subnet_state = Some(SubnetStateOps::export_view());
        self
    }

    #[must_use]
    pub fn with_app_directory(mut self) -> Self {
        self.snapshot.app_directory = Some(AppDirectoryOps::export_view());
        self
    }

    #[must_use]
    pub fn with_subnet_directory(mut self) -> Self {
        self.snapshot.subnet_directory = Some(SubnetDirectoryOps::export_view());
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
        let parents = SubnetRegistryOps::parent_chain_view(target_pid)?;
        let mut children_map = HashMap::new();

        for parent in &parents {
            let children = SubnetRegistryOps::children_child_view(parent.pid);
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
