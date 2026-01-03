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
    Error,
    access::env,
    ids::CanisterRole,
    ops::storage::{
        directory::{app::AppDirectorySnapshot, subnet::SubnetDirectorySnapshot},
        registry::subnet::SubnetRegistryOps,
        state::{
            app::{AppStateOps, AppStateSnapshot},
            subnet::{self, SubnetStateSnapshot},
        },
    },
    workflow::{
        canister::mapper::CanisterMapper,
        children::mapper::ChildrenMapper,
        directory::{AppDirectoryResolver, SubnetDirectoryResolver},
    },
};
use candid::Principal;
use std::collections::HashMap;

///
/// StateSnapshot
/// Internal workflow snapshot (not a DTO)
///

#[derive(Default)]
pub struct StateSnapshot {
    pub app_state: Option<AppStateSnapshot>,
    pub subnet_state: Option<SubnetStateSnapshot>,
    pub app_directory: Option<AppDirectorySnapshot>,
    pub subnet_directory: Option<SubnetDirectorySnapshot>,
}

///
/// StateSnapshotBuilder
///
/// Assembles internal `StateSnapshot` values from authoritative state.
/// DTO conversion happens via `From<StateSnapshot> for StateSnapshotView`.
/// Root-only; construction enforces root context.
///

pub struct StateSnapshotBuilder {
    snapshot: StateSnapshot,
}

impl StateSnapshotBuilder {
    pub fn new() -> Result<Self, Error> {
        env::require_root()?;

        Ok(Self {
            snapshot: StateSnapshot::default(),
        })
    }

    #[must_use]
    pub fn with_app_state(mut self) -> Self {
        self.snapshot.app_state = Some(AppStateOps::snapshot());
        self
    }

    #[must_use]
    pub fn with_subnet_state(mut self) -> Self {
        self.snapshot.subnet_state = Some(subnet::snapshot());
        self
    }

    #[must_use]
    pub fn with_app_directory(mut self) -> Self {
        self.snapshot.app_directory = Some(AppDirectoryResolver::resolve());
        self
    }

    #[must_use]
    pub fn with_subnet_directory(mut self) -> Self {
        self.snapshot.subnet_directory = Some(SubnetDirectoryResolver::resolve());
        self
    }

    #[must_use]
    pub fn build(self) -> StateSnapshot {
        self.snapshot
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
    pub(crate) fn for_target(target_pid: Principal) -> Result<Self, Error> {
        let registry_snapshot = SubnetRegistryOps::snapshot();

        let parents: Vec<TopologyPathNode> = registry_snapshot
            .parent_chain(target_pid)?
            .into_iter()
            .map(|(pid, summary)| {
                let node = CanisterMapper::summary_to_topology_node(pid, &summary);
                TopologyPathNode {
                    pid: node.pid,
                    role: node.role,
                    parent_pid: node.parent_pid,
                }
            })
            .collect();

        let mut children_map = HashMap::new();

        for parent in &parents {
            let child_snapshot =
                ChildrenMapper::from_registry_snapshot(&registry_snapshot, parent.pid);

            let children: Vec<TopologyDirectChild> = child_snapshot
                .entries
                .into_iter()
                .map(|child| TopologyDirectChild {
                    pid: child.pid,
                    role: child.role,
                })
                .collect();

            children_map.insert(parent.pid, children);
        }

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
    snapshot.app_state.is_none()
        && snapshot.subnet_state.is_none()
        && snapshot.app_directory.is_none()
        && snapshot.subnet_directory.is_none()
}

#[must_use]
pub fn state_snapshot_debug(snapshot: &StateSnapshot) -> String {
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
