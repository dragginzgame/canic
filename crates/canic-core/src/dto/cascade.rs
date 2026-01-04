use crate::dto::{
    prelude::*,
    state::{AppStateView, SubnetStateView},
    topology::{AppDirectoryView, SubnetDirectoryView},
};

///
/// StateSnapshotView
/// Snapshot of mutable state and directory sections that can be propagated to peers.
/// Pure DTO.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct StateSnapshotView {
    pub app_state: Option<AppStateView>,
    pub subnet_state: Option<SubnetStateView>,
    pub app_directory: Option<AppDirectoryView>,
    pub subnet_directory: Option<SubnetDirectoryView>,
}

///
/// TopologySnapshotView
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
pub struct TopologySnapshotView {
    pub parents: Vec<TopologyPathNodeView>,
    /// Children keyed by their parent pid (at most one entry per parent).
    pub children_map: Vec<TopologyChildrenView>,
}

///
/// TopologyChildrenView
/// Parent-keyed children list used in topology cascades.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TopologyChildrenView {
    pub parent_pid: Principal,
    pub children: Vec<TopologyDirectChildView>,
}

///
/// TopologyDirectChildView
/// Direct child node for parent-keyed topology maps.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TopologyDirectChildView {
    pub pid: Principal,
    pub role: CanisterRole,
}

///
/// TopologyPathNodeView
/// Snapshot node for parent-path traversal (includes identity).
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TopologyPathNodeView {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
}
