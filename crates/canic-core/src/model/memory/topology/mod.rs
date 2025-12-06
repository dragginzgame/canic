mod app;
mod subnet;

pub use app::*;
pub use subnet::*;

use crate::ids::SubnetRole;
use candid::{CandidType, Principal};
use serde::Deserialize;

///
/// SubnetIdentity
///

#[derive(CandidType, Debug, Deserialize)]
pub enum SubnetIdentity {
    Prime,

    // this subnet is general-purpose subnet that syncs from Prime
    Standard(SubnetContextParams),

    // explicitly provided subnet principal (test/support only)
    Manual(Principal),
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
