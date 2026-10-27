//
// StateSnapshotInput
//
// Cascade state snapshot.
//

use crate::dto::{prelude::*, state::FleetStateInput};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct StateSnapshotInput {
    pub fleet_state: Option<FleetStateInput>,
}

/// One observed result; an unconfirmed call may have applied before its reply was lost.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum StateCascadeTargetResult {
    Applied,
    AppliedWithReconciliationFailure(crate::dto::error::Error),
    Unconfirmed(crate::dto::error::Error),
}

/// Exact canister associated with one bounded state-cascade observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct StateCascadeTargetOutcome {
    pub canister_id: Principal,
    pub result: StateCascadeTargetResult,
}

/// Aggregate observations retain full counts when the bounded detail list fills.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct StateCascadeReport {
    pub successful_targets: u64,
    pub reconciliation_failures: u64,
    pub unconfirmed_targets: u64,
    pub omitted_targets: u64,
    pub targets: Vec<StateCascadeTargetOutcome>,
}

//
// TopologySnapshotInput
//
// Cascade topology snapshot.
//

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
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

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct TopologyChildren {
    pub parent_pid: Principal,
    pub children: Vec<TopologyDirectChild>,
}

//
// TopologyDirectChild
//
// Direct child node.
//

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct TopologyDirectChild {
    pub pid: Principal,
    pub role: CanisterRole,
}

//
// TopologyPathNode
//
// Parent-path node.
//

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct TopologyPathNode {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
}
