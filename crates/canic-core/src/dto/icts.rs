use crate::dto::prelude::*;

///
/// CanisterMetadataResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterMetadataResponse {
    pub name: String,
    pub version: String,
    pub description: String,
}
