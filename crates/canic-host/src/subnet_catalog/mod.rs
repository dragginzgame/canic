//! Module: subnet_catalog
//!
//! Responsibility: bind Canic's host-only Subnet Catalog cache and refresh authority.
//! Does not own: Subnet classification, Registry collection, or placement policy.
//! Boundary: callers receive only the validated catalog produced by `ic-query`.

pub mod acquisition;
mod evidence;
pub mod ops;
#[cfg(test)]
mod tests;
pub mod view;

use ic_query::subnet_catalog::{
    CatalogAssurance, CatalogLoadOutcome, CatalogSourceSelection,
    DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT, MAINNET_NETWORK, SubnetCatalogCacheRequest,
    SubnetCatalogLoadFailure, SubnetCatalogLoadRequest, load_cached_subnet_catalog_detailed,
};
use std::path::{Path, PathBuf};

pub use acquisition::MainnetCatalogClient;

pub use evidence::{
    SubnetCatalogFailureCacheDispositionV1, SubnetCatalogFailureEffectsV1, SubnetCatalogFieldV1,
    SubnetCatalogLoadFailureEvidenceV1, SubnetCatalogLoadStageV1, SubnetCatalogRefreshTriggerV1,
    SubnetCatalogRegistryRecordEvidenceV1, SubnetCatalogRegistryRecordKindV1,
    SubnetCatalogRegistryValueEncodingV1, SubnetCatalogRetryabilityV1, SubnetCatalogSourceKindV1,
    SubnetCatalogSubjectV1, SubnetCatalogUnknownRetryReasonV1,
};

const IC_QUERY_CACHE_DIRECTORY: &str = "ic-query";

/// Maximum accepted catalog age when generating new mainnet desired state.
pub const MAINNET_CATALOG_MAX_AGE_SECONDS: u64 = 3_600;

/// Whole-acquisition deadline, shared by both endpoint collections and assurance repair.
pub const MAINNET_CATALOG_ACQUISITION_SECONDS: u64 = 600;

/// Distinct API hostnames whose Registry version and canonical payload must agree.
/// This is endpoint agreement, not independent-provider or certified evidence.
pub const MAINNET_CATALOG_ENDPOINTS: [&str; 2] =
    ["https://ic0.app", DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT];

/// Load existing validated mainnet evidence without a network call or cache mutation.
pub fn load_cached_mainnet_subnet_catalog(
    icp_root: &Path,
    now_unix_secs: u64,
) -> Result<CatalogLoadOutcome, Box<SubnetCatalogLoadFailure>> {
    let request = mainnet_subnet_catalog_cache_only_request(icp_root, now_unix_secs);
    load_cached_subnet_catalog_detailed(&request)
}

fn mainnet_subnet_catalog_load_request(
    icp_root: &Path,
    now_unix_secs: u64,
) -> SubnetCatalogLoadRequest {
    let cache = SubnetCatalogCacheRequest::new(
        mainnet_subnet_catalog_cache_root(icp_root),
        MAINNET_NETWORK,
    );
    SubnetCatalogLoadRequest::refresh_missing_invalid_or_older_than(
        cache,
        mainnet_source_selection(),
        now_unix_secs,
        MAINNET_CATALOG_MAX_AGE_SECONDS,
    )
    .with_minimum_assurance(CatalogAssurance::MultiEndpointAgreement)
}

fn mainnet_source_selection() -> CatalogSourceSelection {
    CatalogSourceSelection::multi_endpoint_agreement(
        MAINNET_CATALOG_ENDPOINTS
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
    )
}

fn mainnet_subnet_catalog_cache_only_request(
    icp_root: &Path,
    now_unix_secs: u64,
) -> SubnetCatalogLoadRequest {
    let cache = SubnetCatalogCacheRequest::new(
        mainnet_subnet_catalog_cache_root(icp_root),
        MAINNET_NETWORK,
    );
    SubnetCatalogLoadRequest::cache_only(cache, now_unix_secs)
        .with_minimum_assurance(CatalogAssurance::MultiEndpointAgreement)
}

/// Return the private capability root used for Canic's embedded `ic-query` cache.
#[must_use]
pub fn mainnet_subnet_catalog_cache_root(icp_root: &Path) -> PathBuf {
    icp_root.join(".canic").join(IC_QUERY_CACHE_DIRECTORY)
}
