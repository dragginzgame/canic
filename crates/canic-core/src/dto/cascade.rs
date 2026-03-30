use crate::dto::{
    prelude::*,
    state::{AppStateInput, SubnetStateInput},
    topology::{AppDirectoryArgs, SubnetDirectoryArgs},
};

///
/// StateSnapshotInput
/// Snapshot of mutable state and directory sections that can be propagated to peers.
/// Pure DTO.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct StateSnapshotInput {
    pub app_state: Option<AppStateInput>,
    pub subnet_state: Option<SubnetStateInput>,
    pub app_directory: Option<AppDirectoryArgs>,
    pub subnet_directory: Option<SubnetDirectoryArgs>,
}

///
/// TopologySnapshotInput
/// Partial topology snapshot used for cascade.
/// Contains:
/// - a parent path (root -> target)
/// - direct children for each node on that path only
///
/// This is not a full topology export.
///
/// Pure DTO.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TopologySnapshotInput {
    pub parents: Vec<TopologyPathNode>,
    /// Children keyed by their parent pid (at most one entry per parent).
    pub children_map: Vec<TopologyChildren>,
}

///
/// TopologyChildren
/// Parent-keyed children list used in topology cascades.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TopologyChildren {
    pub parent_pid: Principal,
    pub children: Vec<TopologyDirectChild>,
}

///
/// TopologyDirectChild
/// Direct child node for parent-keyed topology maps.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TopologyDirectChild {
    pub pid: Principal,
    pub role: CanisterRole,
}

///
/// TopologyPathNode
/// Snapshot node for parent-path traversal (includes identity).
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TopologyPathNode {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
}
