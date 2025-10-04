use crate::spec::prelude::*;

///
/// GetSubnetForCanisterResponse
///

#[derive(CandidType, Deserialize, Debug)]
pub struct GetSubnetForCanisterResponse {
    pub subnet_id: Principal,
}
