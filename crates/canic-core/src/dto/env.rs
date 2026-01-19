use crate::dto::prelude::*;

///
/// EnvBootstrapArgs
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct EnvBootstrapArgs {
    // app
    pub prime_root_pid: Option<Principal>,

    // subnet
    pub subnet_role: Option<SubnetRole>,
    pub subnet_pid: Option<Principal>,
    pub root_pid: Option<Principal>,

    // canister
    pub canister_role: Option<CanisterRole>,
    pub parent_pid: Option<Principal>,
}

///
/// EnvSnapshotResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct EnvSnapshotResponse {
    // app
    pub prime_root_pid: Option<Principal>,

    // subnet
    pub subnet_role: Option<SubnetRole>,
    pub subnet_pid: Option<Principal>,
    pub root_pid: Option<Principal>,

    // canister
    pub canister_role: Option<CanisterRole>,
    pub parent_pid: Option<Principal>,
}
