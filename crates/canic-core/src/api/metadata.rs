//! Module: api::metadata
//!
//! Responsibility: public Canic metadata facade for endpoint callers.
//! Does not own: package metadata generation or endpoint macro emission.
//! Boundary: builds metadata DTOs from package and canister version inputs.

use crate::dto::metadata::CanicMetadataResponse;

///
/// CanicMetadataApi
///
/// Thin endpoint-facing facade for Canic metadata responses.
///

pub struct CanicMetadataApi;

impl CanicMetadataApi {
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
