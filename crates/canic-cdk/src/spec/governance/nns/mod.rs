//!
//! Handy wrappers for selected NNS candid types with consistent naming.
//!

use crate::spec::prelude::*;

///
/// GetSubnetForCanisterRequest
///

#[derive(CandidType, Debug, Deserialize)]
pub struct GetSubnetForCanisterRequest {
    pub principal: Principal,
}

///
/// GetSubnetForCanisterResponse
/// Minimal NNS response describing the assigned subnet for a canister.
///

pub type GetSubnetForCanisterResponse = Result<GetSubnetForCanisterPayload, String>;

#[derive(CandidType, Debug, Deserialize)]
pub struct GetSubnetForCanisterPayload {
    pub subnet_id: Option<Principal>,
}
