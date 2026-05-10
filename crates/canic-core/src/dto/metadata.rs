use crate::dto::prelude::*;

//
// CanicMetadataResponse
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CanicMetadataResponse {
    pub package_name: String,
    pub package_version: String,
    pub package_description: String,
    pub canic_version: String,
    pub canister_version: u64,
}
