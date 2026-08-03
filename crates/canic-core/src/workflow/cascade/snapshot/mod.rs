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
    dto::{cascade::StateSnapshotInput, state::FleetStateInput, topology::FleetDirectoryInput},
    ids::CanisterRole,
    ops::{
        runtime::env::EnvOps, storage::state::fleet::FleetStateOps,
        topology::directory::FleetDirectoryResolver,
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
        self.snapshot.fleet_directory = Some(FleetDirectoryResolver::resolve_input()?);
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
    pub(crate) fn for_direct_leaf(
        root_pid: Principal,
        target_pid: Principal,
        target_role: CanisterRole,
    ) -> Result<Self, InternalError> {
        if target_pid == Principal::anonymous() {
            return Err(InternalError::invalid_input(
                "Fleet activation child Canister is anonymous",
            ));
        }
        if target_pid == root_pid {
            return Err(InternalError::invalid_input(
                "Fleet activation child equals the Fleet Subnet Root",
            ));
        }

        Ok(Self {
            snapshot: TopologySnapshot {
                parents: vec![
                    TopologyPathNode {
                        pid: root_pid,
                        role: CanisterRole::ROOT,
                        parent_pid: None,
                    },
                    TopologyPathNode {
                        pid: target_pid,
                        role: target_role.clone(),
                        parent_pid: Some(root_pid),
                    },
                ],
                children_map: HashMap::from([
                    (
                        root_pid,
                        vec![TopologyDirectChild {
                            pid: target_pid,
                            role: target_role,
                        }],
                    ),
                    (target_pid, Vec::new()),
                ]),
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
    snapshot.fleet_state.is_none() && snapshot.fleet_directory.is_none()
}

#[must_use]
pub fn state_snapshot_debug(snapshot: &StateSnapshot) -> String {
    const fn fmt(present: bool, code: &str) -> &str {
        if present { code } else { ".." }
    }

    format!(
        "[{} {}]",
        fmt(snapshot.fleet_state.is_some(), "fs"),
        fmt(snapshot.fleet_directory.is_some(), "fd"),
    )
}

#[cfg(test)]
mod tests {
    use super::{StateSnapshot, TopologySnapshotBuilder};
    use crate::cdk::types::Principal;
    use crate::dto::state::{FleetMode, FleetStateInput};
    use crate::ids::CanisterRole;

    #[test]
    fn state_snapshot_debug_reports_current_slots() {
        let snapshot = StateSnapshot {
            fleet_state: Some(FleetStateInput {
                mode: FleetMode::Enabled,
                cycles_funding_enabled: true,
            }),
            fleet_directory: None,
        };

        assert_eq!(super::state_snapshot_debug(&snapshot), "[fs ..]");
    }

    #[test]
    fn direct_leaf_topology_has_one_root_child_and_no_descendants() {
        let root = Principal::from_slice(&[1]);
        let store = Principal::from_slice(&[2]);
        let snapshot =
            TopologySnapshotBuilder::for_direct_leaf(root, store, CanisterRole::WASM_STORE)
                .expect("direct leaf topology")
                .build();

        assert_eq!(snapshot.parents.len(), 2);
        assert_eq!(snapshot.parents[0].pid, root);
        assert_eq!(snapshot.parents[1].pid, store);
        assert_eq!(snapshot.parents[1].parent_pid, Some(root));
        assert_eq!(snapshot.children_map[&root].len(), 1);
        assert_eq!(snapshot.children_map[&root][0].pid, store);
        assert!(snapshot.children_map[&store].is_empty());
    }
}
