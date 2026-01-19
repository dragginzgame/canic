//!
//! Snapshot ↔ DTO adapters.
//!
//! This module owns conversion between:
//! - internal workflow snapshots (`StateSnapshot`, `TopologySnapshot`)
//! - transport DTOs (`StateSnapshotInput`, `TopologySnapshotInput`)
//!
//! RULES:
//! - No assembly logic here
//! - No ops calls
//! - No persistence
//! - This is the *only* place workflow converts snapshot DTOs
//!

use crate::{
    dto::cascade::{
        StateSnapshotInput, TopologyChildren, TopologyDirectChild as TopologyDirectChildDto,
        TopologyPathNode as TopologyPathNodeDto, TopologySnapshotInput,
    },
    workflow::cascade::snapshot::{
        StateSnapshot, TopologyDirectChild, TopologyPathNode, TopologySnapshot,
    },
};

///
/// StateSnapshotAdapter
///

pub struct StateSnapshotAdapter;

impl StateSnapshotAdapter {
    #[must_use]
    pub fn to_input(snapshot: &StateSnapshot) -> StateSnapshotInput {
        StateSnapshotInput {
            app_state: snapshot.app_state,
            subnet_state: snapshot.subnet_state,
            app_directory: snapshot.app_directory.clone(),
            subnet_directory: snapshot.subnet_directory.clone(),
        }
    }

    #[must_use]
    pub fn from_input(view: StateSnapshotInput) -> StateSnapshot {
        StateSnapshot::from(view)
    }
}

///
/// TopologySnapshotAdapter
///

pub struct TopologySnapshotAdapter;

impl TopologySnapshotAdapter {
    #[must_use]
    pub fn to_input(snapshot: &TopologySnapshot) -> TopologySnapshotInput {
        TopologySnapshotInput {
            parents: snapshot
                .parents
                .iter()
                .map(|p| TopologyPathNodeDto {
                    pid: p.pid,
                    role: p.role.clone(),
                    parent_pid: p.parent_pid,
                })
                .collect(),

            children_map: snapshot
                .children_map
                .iter()
                .map(|(pid, children)| TopologyChildren {
                    parent_pid: *pid,
                    children: children
                        .iter()
                        .map(|c| TopologyDirectChildDto {
                            pid: c.pid,
                            role: c.role.clone(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    #[must_use]
    pub fn from_input(view: TopologySnapshotInput) -> TopologySnapshot {
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
                .map(|entry| {
                    let mapped = entry
                        .children
                        .into_iter()
                        .map(|child| TopologyDirectChild {
                            pid: child.pid,
                            role: child.role,
                        })
                        .collect();
                    (entry.parent_pid, mapped)
                })
                .collect(),
        }
    }
}
