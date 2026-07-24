use crate::dto::prelude::*;

//
// EnvBootstrapArgs
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct EnvBootstrapArgs {
    // fleet
    pub fleet_root_pid: Option<Principal>,

    // subnet
    pub subnet_slot: Option<SubnetSlotId>,
    pub subnet_pid: Option<Principal>,
    pub root_pid: Option<Principal>,

    // canister
    pub canister_role: Option<CanisterRole>,
    pub parent_pid: Option<Principal>,
}

//
// EnvSnapshotResponse
//

#[derive(CandidType, Deserialize)]
pub struct EnvSnapshotResponse {
    // fleet
    pub fleet_root_pid: Option<Principal>,

    // subnet
    pub subnet_slot: Option<SubnetSlotId>,
    pub subnet_pid: Option<Principal>,
    pub root_pid: Option<Principal>,

    // canister
    pub canister_role: Option<CanisterRole>,
    pub parent_pid: Option<Principal>,
}
