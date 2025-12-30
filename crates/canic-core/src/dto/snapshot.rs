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
/// `Default` represents an empty snapshot (all sections omitted). This is used for
/// internal snapshot assembly; it must not be interpreted as an intentional or
/// complete payload.
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

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TopologySnapshotView {
    pub parents: Vec<TopologyNodeView>,
    /// Children keyed by their parent pid (at most one entry per parent).
    pub children_map: HashMap<Principal, Vec<TopologyChildView>>,
}

///
/// TopologyChildView
/// Child node for parent-keyed topology maps.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TopologyChildView {
    pub pid: Principal,
    pub role: CanisterRole,
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
