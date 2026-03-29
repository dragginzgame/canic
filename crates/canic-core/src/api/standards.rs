use crate::dto::standards::CanicStandardsResponse;

const CANISTER_NAME: &str = env!("CARGO_PKG_NAME");
const CANISTER_VERSION: &str = env!("CARGO_PKG_VERSION");
const CANISTER_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

///
/// CanicStandardsApi
///

pub struct CanicStandardsApi;

impl CanicStandardsApi {
    #[must_use]
    pub fn metadata() -> CanicStandardsResponse {
        CanicStandardsResponse {
            name: CANISTER_NAME.to_string(),
            version: CANISTER_VERSION.to_string(),
            description: CANISTER_DESCRIPTION.to_string(),
        }
    }
}
