//!
//! Handy wrappers for selected NNS candid types with consistent naming.
//!

use crate::spec::prelude::*;

///
/// GetSubnetForCanisterRequest
///

#[derive(CandidType, Deserialize, Debug)]
pub struct GetSubnetForCanisterRequest {
    pub principal: Principal,
}

impl GetSubnetForCanisterRequest {
    pub fn new(pid: impl Into<Principal>) -> Self {
        Self {
            principal: pid.into(),
        }
    }
}

///
/// GetSubnetForCanisterResponse
/// Minimal NNS response describing the assigned subnet for a canister.
///

pub type GetSubnetForCanisterResponse = Result<GetSubnetForCanisterPayload, String>;

#[derive(CandidType, Deserialize, Debug)]
pub struct GetSubnetForCanisterPayload {
    pub subnet_id: Option<Principal>,
}
