use crate::dto::icts::CanisterMetadataResponse;

///
/// IctsApi
///

pub struct IctsApi;

impl IctsApi {
    #[must_use]
    pub fn name() -> String {
        env!("CARGO_PKG_NAME").to_string()
    }

    #[must_use]
    pub fn version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    #[must_use]
    pub fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").to_string()
    }

    #[must_use]
    pub fn metadata() -> CanisterMetadataResponse {
        CanisterMetadataResponse {
            name: Self::name(),
            version: Self::version(),
            description: Self::description(),
        }
    }
}
