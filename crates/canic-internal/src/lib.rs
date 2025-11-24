pub mod canister;

use candid::CandidType;
use canic::types::Principal;
use serde::Deserialize;

///
/// AuthToken
///
/// Example of a user session or access token.
/// This is what the auth canister signed.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct AuthToken {
    pub sub: Principal,
    pub exp: u64,
}
