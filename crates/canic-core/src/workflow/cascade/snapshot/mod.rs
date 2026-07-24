//!
//! Snapshot assembly helpers.
//!
//! This module:
//! - assembles snapshot DTOs from authoritative state via ops
//! - exposes builder-style APIs for snapshot construction
//! - keeps DTOs data-only by placing helper logic here
//!

pub mod adapter;

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::{
        cascade::StateSnapshotInput,
        state::FleetStateInput,
        topology::{FleetDirectoryInput, SubnetDirectoryInput},
    },
    ids::CanisterRole,
    ops::{
        runtime::env::EnvOps,
        storage::{registry::subnet::SubnetRegistryOps, state::fleet::FleetStateOps},
        topology::index::{AppIndexResolver, SubnetIndexResolver},
    },
};
use std::collections::HashMap;

///
/// StateSnapshot
/// Internal workflow snapshot (not a DTO)
///

#[derive(Default)]
pub struct StateSnapshot {
    pub fleet_state: Option<FleetStateInput>,
    pub fleet_directory: Option<FleetDirectoryInput>,
    pub subnet_directory: Option<SubnetDirectoryInput>,
}

///
/// StateSnapshotBuilder
///
/// Assembles internal `StateSnapshot` values from authoritative state.
/// DTO shaping happens in ops; snapshot assembly remains in workflow.
/// Root-only; construction enforces root context.
///

pub struct StateSnapshotBuilder {
    snapshot: StateSnapshot,
}

impl StateSnapshotBuilder {
    pub fn new() -> Result<Self, InternalError> {
        EnvOps::require_root()?;

        Ok(Self {
            snapshot: StateSnapshot::default(),
        })
    }

    #[must_use]
    pub fn with_fleet_state(mut self) -> Self {
        self.snapshot.fleet_state = Some(FleetStateOps::snapshot_input());
        self
    }

    pub fn with_fleet_directory(mut self) -> Result<Self, InternalError> {
        self.snapshot.fleet_directory = Some(AppIndexResolver::resolve_input()?);
        Ok(self)
    }

    pub fn with_subnet_directory(mut self) -> Result<Self, InternalError> {
        self.snapshot.subnet_directory = Some(SubnetIndexResolver::resolve_input()?);
        Ok(self)
    }

    #[must_use]
    pub fn build(self) -> StateSnapshot {
        self.snapshot
    }
}

impl From<StateSnapshotInput> for StateSnapshot {
    fn from(snapshot: StateSnapshotInput) -> Self {
        Self {
            fleet_state: snapshot.fleet_state,
            fleet_directory: snapshot.fleet_directory,
            subnet_directory: snapshot.subnet_directory,
        }
    }
}

///
/// TopologySnapshot
///

#[derive(Clone, Debug)]
pub struct TopologySnapshot {
    pub(crate) parents: Vec<TopologyPathNode>,
    pub(crate) children_map: HashMap<Principal, Vec<TopologyDirectChild>>,
}

///
/// TopologyPathNode
/// Internal representation of a node in the parent chain.
///

#[derive(Clone, Debug)]
pub struct TopologyPathNode {
    pub(crate) pid: Principal,
    pub(crate) role: CanisterRole,
    pub(crate) parent_pid: Option<Principal>,
}

///
/// TopologyDirectChild
/// Internal representation of a direct child.
///

#[derive(Clone, Debug)]
pub struct TopologyDirectChild {
    pub(crate) pid: Principal,
    pub(crate) role: CanisterRole,
}

///
/// TopologySnapshotBuilder
///
/// Workflow helper for assembling topology snapshots.
///

pub struct TopologySnapshotBuilder {
    snapshot: TopologySnapshot,
}

impl TopologySnapshotBuilder {
    pub(crate) fn for_target(target_pid: Principal) -> Result<Self, InternalError> {
        // Build parent chain (root → target)
        let parents: Vec<TopologyPathNode> = SubnetRegistryOps::parent_chain(target_pid)?
            .into_iter()
            .map(|entry| TopologyPathNode {
                pid: entry.pid,
                role: entry.record.role.clone(),
                parent_pid: entry.record.parent_pid,
            })
            .collect();

        // Build direct-children map for each parent in the chain
        let parent_pids: Vec<Principal> = parents.iter().map(|parent| parent.pid).collect();
        let raw_children = SubnetRegistryOps::direct_children_map(&parent_pids);

        let children_map: HashMap<Principal, Vec<TopologyDirectChild>> = raw_children
            .into_iter()
            .map(|(parent_pid, children)| {
                let mapped = children
                    .into_iter()
                    .map(|entry| TopologyDirectChild {
                        pid: entry.pid,
                        role: entry.record.role,
                    })
                    .collect();
                (parent_pid, mapped)
            })
            .collect();

        Ok(Self {
            snapshot: TopologySnapshot {
                parents,
                children_map,
            },
        })
    }

    #[must_use]
    pub fn build(self) -> TopologySnapshot {
        self.snapshot
    }
}

// -----------------------------------------------------------------------------
// Snapshot helpers (workflow-owned)
// -----------------------------------------------------------------------------

#[must_use]
pub const fn state_snapshot_is_empty(snapshot: &StateSnapshot) -> bool {
    snapshot.fleet_state.is_none()
        && snapshot.fleet_directory.is_none()
        && snapshot.subnet_directory.is_none()
}

#[must_use]
pub fn state_snapshot_debug(snapshot: &StateSnapshot) -> String {
    const fn fmt(present: bool, code: &str) -> &str {
        if present { code } else { ".." }
    }

    format!(
        "[{} {} {}]",
        fmt(snapshot.fleet_state.is_some(), "fs"),
        fmt(snapshot.fleet_directory.is_some(), "fd"),
        fmt(snapshot.subnet_directory.is_some(), "sd"),
    )
}

#[cfg(test)]
mod tests {
    use super::StateSnapshot;
    use crate::dto::state::{FleetMode, FleetStateInput};

    #[test]
    fn state_snapshot_debug_reports_current_slots() {
        let snapshot = StateSnapshot {
            fleet_state: Some(FleetStateInput {
                mode: FleetMode::Enabled,
                cycles_funding_enabled: true,
            }),
            fleet_directory: None,
            subnet_directory: None,
        };

        assert_eq!(super::state_snapshot_debug(&snapshot), "[fs .. ..]");
    }
}
