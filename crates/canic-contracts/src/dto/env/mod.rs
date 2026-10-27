//
// EnvBootstrapArgs
//

use crate::dto::prelude::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct EnvBootstrapArgs {
    // fleet
    pub fleet_subnet_root_pid: Option<Principal>,

    // Component declaration and physical Subnet placement
    pub component_spec: Option<ComponentSpecId>,
    pub subnet_pid: Option<Principal>,
    pub root_pid: Option<Principal>,

    // canister
    pub canister_role: Option<CanisterRole>,
    pub parent_pid: Option<Principal>,
}

//
// EnvSnapshotResponse
//

#[derive(Deserialize, serde::Serialize)]
pub struct EnvSnapshotResponse {
    // fleet
    pub fleet_subnet_root_pid: Option<Principal>,

    // Component declaration and physical Subnet placement
    pub component_spec: Option<ComponentSpecId>,
    pub subnet_pid: Option<Principal>,
    pub root_pid: Option<Principal>,

    // canister
    pub canister_role: Option<CanisterRole>,
    pub parent_pid: Option<Principal>,
}
