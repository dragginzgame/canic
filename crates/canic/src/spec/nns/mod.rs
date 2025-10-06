//!
//! Handy wrappers for selected NNS candid types with consistent naming.
//!

use crate::spec::prelude::*;

///
/// GetSubnetForCanisterResponse
/// Minimal NNS response describing the assigned subnet for a canister.
///

#[derive(CandidType, Deserialize, Debug)]
pub struct GetSubnetForCanisterResponse {
    pub subnet_id: Principal,
}
