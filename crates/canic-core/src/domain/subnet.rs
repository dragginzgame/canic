//! Module: domain::subnet
//!
//! Responsibility: define runtime subnet identity values used during root
//! lifecycle initialization.
//! Does not own: config parsing, endpoint DTO structs, or stable subnet
//! records.
//! Boundary: DTOs re-export these values for the init-argument Candid boundary
//! while workflow consumes the domain owner directly.

use crate::{cdk::types::Principal, ids::SubnetSlotId};
use candid::CandidType;
use serde::Deserialize;

//
// SubnetIdentity
//
// Represents the *runtime identity* of the subnet this canister is executing in.
// Must never be constructed from configuration alone.
//

#[derive(CandidType, Debug, Deserialize)]
pub enum SubnetIdentity {
    Prime,

    PrimeWithModuleHash(Vec<u8>),

    // this subnet is general-purpose subnet that syncs from Prime
    Standard(SubnetContextParams),

    // do not attempt subnet discovery (test / support mode)
    Manual,
}

//
// SubnetContextParams
// everything we need to populate the SubnetContext on a non-Prime subnet
//

#[derive(CandidType, Debug, Deserialize)]
pub struct SubnetContextParams {
    pub subnet_type: SubnetSlotId,
    pub prime_root_pid: Principal,
}
