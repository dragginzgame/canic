//
// CanicMetadataResponse
//

use crate::dto::prelude::*;

#[derive(CandidType, Clone, Debug, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct CanicMetadataResponse {
    pub package_name: String,
    pub package_version: String,
    pub package_description: String,
    pub canic_version: String,
    pub canister_version: u64,
}
