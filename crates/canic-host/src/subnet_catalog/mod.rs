//! Module: subnet_catalog
//!
//! Responsibility: bind Canic's host-only Subnet Catalog cache and refresh authority.
//! Does not own: Subnet classification, Registry collection, or placement policy.
//! Boundary: callers receive only the validated catalog produced by `ic-query`.

use ic_query::subnet_catalog::{
    CatalogLoadOutcome, DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT, MAINNET_NETWORK,
    SubnetCatalogCacheRequest, SubnetCatalogHostError, SubnetCatalogLoadRequest,
    load_subnet_catalog,
};
use std::path::{Path, PathBuf};

const IC_QUERY_CACHE_DIRECTORY: &str = "ic-query";

/// Load Canic's validated mainnet Subnet Catalog under an explicit repair policy.
pub fn load_mainnet_subnet_catalog(
    icp_root: &Path,
    now_unix_secs: u64,
) -> Result<CatalogLoadOutcome, SubnetCatalogHostError> {
    let cache = SubnetCatalogCacheRequest::new(
        mainnet_subnet_catalog_cache_root(icp_root),
        MAINNET_NETWORK,
    );
    let request = SubnetCatalogLoadRequest::refresh_missing_or_invalid(
        cache,
        DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT,
        now_unix_secs,
    );
    load_subnet_catalog(&request)
}

/// Return the private capability root used for Canic's embedded `ic-query` cache.
#[must_use]
pub fn mainnet_subnet_catalog_cache_root(icp_root: &Path) -> PathBuf {
    icp_root.join(".canic").join(IC_QUERY_CACHE_DIRECTORY)
}
