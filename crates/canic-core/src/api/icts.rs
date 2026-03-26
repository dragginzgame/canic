use crate::dto::icts::CanisterMetadataResponse;

const CANISTER_NAME: &str = env!("CARGO_PKG_NAME");
const CANISTER_VERSION: &str = env!("CARGO_PKG_VERSION");
const CANISTER_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

///
/// IctsApi
///

pub struct IctsApi;

impl IctsApi {
    #[must_use]
    pub fn name() -> String {
        CANISTER_NAME.to_string()
    }

    #[must_use]
    pub fn version() -> String {
        CANISTER_VERSION.to_string()
    }

    #[must_use]
    pub fn description() -> String {
        CANISTER_DESCRIPTION.to_string()
    }

    #[must_use]
    pub fn metadata() -> CanisterMetadataResponse {
        CanisterMetadataResponse {
            name: CANISTER_NAME.to_string(),
            version: CANISTER_VERSION.to_string(),
            description: CANISTER_DESCRIPTION.to_string(),
        }
    }
}
