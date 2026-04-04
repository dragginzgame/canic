use crate::dto::{
    prelude::*,
    state::{AppStateInput, SubnetStateInput},
    topology::{AppDirectoryArgs, SubnetDirectoryArgs},
};

//
// StateSnapshotInput
//
// Cascade state snapshot.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct StateSnapshotInput {
    pub app_state: Option<AppStateInput>,
    pub subnet_state: Option<SubnetStateInput>,
    pub app_directory: Option<AppDirectoryArgs>,
    pub subnet_directory: Option<SubnetDirectoryArgs>,
}

//
// TopologySnapshotInput
//
// Cascade topology snapshot.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TopologySnapshotInput {
    pub parents: Vec<TopologyPathNode>,
    // Children keyed by parent pid.
    pub children_map: Vec<TopologyChildren>,
}

//
// TopologyChildren
//
// Parent-keyed child list.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TopologyChildren {
    pub parent_pid: Principal,
    pub children: Vec<TopologyDirectChild>,
}

//
// TopologyDirectChild
//
// Direct child node.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TopologyDirectChild {
    pub pid: Principal,
    pub role: CanisterRole,
}

//
// TopologyPathNode
//
// Parent-path node.
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TopologyPathNode {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
}
