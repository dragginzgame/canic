use crate::dto::prelude::*;

//
// CanicStandardsResponse
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct CanicStandardsResponse {
    pub name: String,
    pub version: String,
    pub description: String,
}
