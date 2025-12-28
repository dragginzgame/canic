use crate::dto::{
    directory::{AppDirectoryView, SubnetDirectoryView},
    prelude::*,
    state::{AppStateView, SubnetStateView},
};

///
/// StateSnapshotView
/// Snapshot of mutable state and directory sections that can be propagated to peers.
/// Pure DTO.
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
pub struct StateSnapshotView {
    // states
    pub app_state: Option<AppStateView>,
    pub subnet_state: Option<SubnetStateView>,

    // directories
    pub app_directory: Option<AppDirectoryView>,
    pub subnet_directory: Option<SubnetDirectoryView>,
}

///
/// TopologySnapshotView
/// Snapshot of canister topology relationships.
/// Pure DTO.
///

#[derive(CandidType, Clone, Debug, Default, Deserialize)]
pub struct TopologySnapshotView {
    pub parents: Vec<TopologyNodeView>,
    pub children_map: HashMap<Principal, Vec<TopologyNodeView>>,
}

///
/// TopologyNodeView
/// Snapshot node for topology traversal (includes identity).
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TopologyNodeView {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
}
