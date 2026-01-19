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
    access::env,
    ops::{
        storage::{
            registry::subnet::SubnetRegistryOps,
            state::{app::AppStateOps, subnet::SubnetStateOps},
        },
        topology::directory::{AppDirectoryResolver, SubnetDirectoryResolver},
    },
    storage::stable::{
        directory::{app::AppDirectoryData, subnet::SubnetDirectoryData},
        state::{app::AppStateData, subnet::SubnetStateData},
    },
    workflow::prelude::*,
};
use std::collections::HashMap;

///
/// StateSnapshot
/// Internal workflow snapshot (not a DTO)
///

#[derive(Default)]
pub struct StateSnapshot {
    pub app_state: Option<AppStateData>,
    pub subnet_state: Option<SubnetStateData>,
    pub app_directory: Option<AppDirectoryData>,
    pub subnet_directory: Option<SubnetDirectoryData>,
}

///
/// StateSnapshotBuilder
///
/// Assembles internal `StateSnapshot` values from authoritative state.
/// DTO conversion happens via `StateSnapshotAdapter`.
/// Root-only; construction enforces root context.
///

pub struct StateSnapshotBuilder {
    snapshot: StateSnapshot,
}

impl StateSnapshotBuilder {
    pub fn new() -> Result<Self, InternalError> {
        env::require_root()?;

        Ok(Self {
            snapshot: StateSnapshot::default(),
        })
    }

    #[must_use]
    pub fn with_app_state(mut self) -> Self {
        self.snapshot.app_state = Some(AppStateOps::data());
        self
    }

    #[must_use]
    pub fn with_subnet_state(mut self) -> Self {
        self.snapshot.subnet_state = Some(SubnetStateOps::data());
        self
    }

    pub fn with_app_directory(mut self) -> Result<Self, InternalError> {
        self.snapshot.app_directory = Some(AppDirectoryResolver::resolve()?);
        Ok(self)
    }

    pub fn with_subnet_directory(mut self) -> Result<Self, InternalError> {
        self.snapshot.subnet_directory = Some(SubnetDirectoryResolver::resolve()?);
        Ok(self)
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
    pub(crate) fn for_target(target_pid: Principal) -> Result<Self, InternalError> {
        let registry_data = SubnetRegistryOps::data();

        // Build parent chain (root → target)
        let parents: Vec<TopologyPathNode> = registry_data
            .parent_chain(target_pid)?
            .into_iter()
            .map(|(pid, record)| TopologyPathNode {
                pid,
                role: record.role.clone(),
                parent_pid: record.parent_pid,
            })
            .collect();

        // Build direct-children map for each parent in the chain
        let mut children_map: HashMap<Principal, Vec<TopologyDirectChild>> = HashMap::new();

        for parent in &parents {
            let children: Vec<TopologyDirectChild> = registry_data
                .entries
                .iter()
                .filter_map(|(pid, record)| {
                    if record.parent_pid == Some(parent.pid) {
                        Some(TopologyDirectChild {
                            pid: *pid,
                            role: record.role.clone(),
                        })
                    } else {
                        None
                    }
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
