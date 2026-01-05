use crate::dto::prelude::*;

///
/// CanisterMetadataView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterMetadataView {
    pub name: String,
    pub version: String,
    pub description: String,
}
