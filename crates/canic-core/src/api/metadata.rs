use crate::dto::metadata::CanicMetadataResponse;

const CANISTER_NAME: &str = env!("CARGO_PKG_NAME");
const CANISTER_VERSION: &str = env!("CARGO_PKG_VERSION");
const CANISTER_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

///
/// CanicMetadataApi
///

pub struct CanicMetadataApi;

impl CanicMetadataApi {
    /// Return metadata for the core crate fallback path.
    #[must_use]
    pub fn metadata(canister_version: u64) -> CanicMetadataResponse {
        Self::metadata_for(
            CANISTER_NAME,
            CANISTER_VERSION,
            CANISTER_DESCRIPTION,
            CANISTER_VERSION,
            canister_version,
        )
    }

    /// Return metadata for the canister crate that exports the endpoint.
    #[must_use]
    pub fn metadata_for(
        package_name: &str,
        package_version: &str,
        package_description: &str,
        canic_version: &str,
        canister_version: u64,
    ) -> CanicMetadataResponse {
        CanicMetadataResponse {
            package_name: package_name.to_string(),
            package_version: package_version.to_string(),
            package_description: package_description.to_string(),
            canic_version: canic_version.to_string(),
            canister_version,
        }
    }
}
