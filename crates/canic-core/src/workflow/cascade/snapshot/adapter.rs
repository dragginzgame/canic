//!
//! Snapshot ↔ DTO adapters.
//!
//! This module owns conversion between:
//! - internal workflow snapshots (`StateSnapshot`, `TopologySnapshot`)
//! - transport DTOs (`StateSnapshotView`, `TopologySnapshotView`)
//!
//! RULES:
//! - No assembly logic here
//! - No ops calls
//! - No persistence
//! - This is the *only* place workflow converts snapshot DTOs
//!

use super::{StateSnapshot, TopologyDirectChild, TopologyPathNode, TopologySnapshot};
use crate::{
    dto::cascade::{
        StateSnapshotView, TopologyDirectChildView, TopologyPathNodeView, TopologySnapshotView,
    },
    workflow::{
        directory::mapper::{AppDirectoryMapper, SubnetDirectoryMapper},
        state::mapper::{AppStateMapper, SubnetStateMapper},
    },
};

//
// -----------------------------------------------------------------------------
// StateSnapshot ↔ StateSnapshotView
// -----------------------------------------------------------------------------

impl From<&StateSnapshot> for StateSnapshotView {
    fn from(snapshot: &StateSnapshot) -> Self {
        Self {
            app_state: snapshot
                .app_state
                .clone()
                .map(AppStateMapper::snapshot_to_view),

            subnet_state: snapshot
                .subnet_state
                .clone()
                .map(SubnetStateMapper::snapshot_to_view),

            app_directory: snapshot
                .app_directory
                .clone()
                .map(AppDirectoryMapper::snapshot_to_view),

            subnet_directory: snapshot
                .subnet_directory
                .clone()
                .map(SubnetDirectoryMapper::snapshot_to_view),
        }
    }
}

impl From<StateSnapshot> for StateSnapshotView {
    fn from(snapshot: StateSnapshot) -> Self {
        Self::from(&snapshot)
    }
}

pub fn state_snapshot_from_view(view: StateSnapshotView) -> StateSnapshot {
    StateSnapshot {
        app_state: view.app_state.map(AppStateMapper::view_to_snapshot),
        subnet_state: view.subnet_state.map(SubnetStateMapper::view_to_snapshot),
        app_directory: view.app_directory.map(AppDirectoryMapper::view_to_snapshot),
        subnet_directory: view
            .subnet_directory
            .map(SubnetDirectoryMapper::view_to_snapshot),
    }
}

//
// -----------------------------------------------------------------------------
// TopologySnapshot ↔ TopologySnapshotView
// -----------------------------------------------------------------------------

impl From<&TopologySnapshot> for TopologySnapshotView {
    fn from(snapshot: &TopologySnapshot) -> Self {
        Self {
            parents: snapshot
                .parents
                .iter()
                .map(|p| TopologyPathNodeView {
                    pid: p.pid,
                    role: p.role.clone(),
                    parent_pid: p.parent_pid,
                })
                .collect(),
            children_map: snapshot
                .children_map
                .iter()
                .map(|(pid, children)| {
                    (
                        *pid,
                        children
                            .iter()
                            .map(|c| TopologyDirectChildView {
                                pid: c.pid,
                                role: c.role.clone(),
                            })
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}

impl From<TopologySnapshot> for TopologySnapshotView {
    fn from(snapshot: TopologySnapshot) -> Self {
        Self::from(&snapshot)
    }
}

pub fn topology_snapshot_from_view(view: TopologySnapshotView) -> TopologySnapshot {
    TopologySnapshot {
        parents: view
            .parents
            .into_iter()
            .map(|p| TopologyPathNode {
                pid: p.pid,
                role: p.role,
                parent_pid: p.parent_pid,
            })
            .collect(),
        children_map: view
            .children_map
            .into_iter()
            .map(|(pid, children)| {
                let mapped = children
                    .into_iter()
                    .map(|child| TopologyDirectChild {
                        pid: child.pid,
                        role: child.role,
                    })
                    .collect();
                (pid, mapped)
            })
            .collect(),
    }
}
