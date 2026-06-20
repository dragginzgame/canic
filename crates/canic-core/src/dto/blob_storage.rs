use crate::dto::prelude::*;

///
/// CreateCertificateResult
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CreateCertificateResult {
    pub method: String,
    pub blob_hash: String,
}
