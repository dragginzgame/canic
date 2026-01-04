use crate::dto::prelude::*;

///
/// SubnetIdentity
///
/// Represents the *runtime identity* of the subnet this canister is executing in.
/// Must never be constructed from configuration alone.
///

#[derive(CandidType, Debug, Deserialize)]
pub enum SubnetIdentity {
    Prime,

    // this subnet is general-purpose subnet that syncs from Prime
    Standard(SubnetContextParams),

    // do not attempt subnet discovery (test / support mode)
    Manual,
}

///
/// SubnetContextParams
/// everything we need to populate the SubnetContext on a non-Prime subnet
///

#[derive(CandidType, Debug, Deserialize)]
pub struct SubnetContextParams {
    pub subnet_type: SubnetRole,
    pub prime_root_pid: Principal,
}
