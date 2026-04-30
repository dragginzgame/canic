use crate::dto::standards::CanicStandardsResponse;

const CANISTER_NAME: &str = env!("CARGO_PKG_NAME");
const CANISTER_VERSION: &str = env!("CARGO_PKG_VERSION");
const CANISTER_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

///
/// CanicStandardsApi
///

pub struct CanicStandardsApi;

impl CanicStandardsApi {
    /// Return standards metadata for the core crate fallback path.
    #[must_use]
    pub fn metadata() -> CanicStandardsResponse {
        Self::metadata_for(CANISTER_NAME, CANISTER_VERSION, CANISTER_DESCRIPTION)
    }

    /// Return standards metadata for the canister crate that exports the endpoint.
    #[must_use]
    pub fn metadata_for(name: &str, version: &str, description: &str) -> CanicStandardsResponse {
        CanicStandardsResponse {
            name: name.to_string(),
            version: version.to_string(),
            description: description.to_string(),
        }
    }
}
