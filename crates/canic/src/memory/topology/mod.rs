mod app;
mod subnet;

pub use app::*;
pub use subnet::*;

use crate::{
    Error, ThisError,
    memory::MemoryError,
    types::{CanisterType, SubnetType},
};
use candid::{CandidType, Principal};
use serde::Deserialize;

///
/// TopologyError
///

#[derive(Debug, ThisError)]
pub enum TopologyError {
    #[error("canister already installed: {0}")]
    CanisterAlreadyInstalled(Principal),

    #[error("canister not found: {0}")]
    PrincipalNotFound(Principal),

    #[error("subnet not found: {0}")]
    SubnetNotFound(Principal),

    #[error("canister not found: {0}")]
    TypeNotFound(CanisterType),
}

impl From<TopologyError> for Error {
    fn from(err: TopologyError) -> Self {
        MemoryError::from(err).into()
    }
}

///
/// SubnetIdentity
///

#[derive(Debug, CandidType, Deserialize)]
pub enum SubnetIdentity {
    Prime,

    // this subnet is general-purpose subnet that syncs from Prime
    Standard(SubnetContextParams),
}

///
/// SubnetContextParams
/// everything we need to populate the SubnetContext on a non-Prime subnet
///

#[derive(Debug, CandidType, Deserialize)]
pub struct SubnetContextParams {
    pub subnet_type: SubnetType,
    pub prime_root_pid: Principal,
}
