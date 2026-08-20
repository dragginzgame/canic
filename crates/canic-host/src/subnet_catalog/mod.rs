//! Module: subnet_catalog
//!
//! Responsibility: bind Canic's host-only Subnet Catalog cache and refresh authority.
//! Does not own: Subnet classification, Registry collection, or placement policy.
//! Boundary: callers receive only the validated catalog produced by `ic-query`.

use ic_query::subnet_catalog::{
    CatalogAssurance, CatalogLoadOutcome, CatalogSourceSelection,
    DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT, MAINNET_NETWORK, SubnetCatalogCacheRequest,
    SubnetCatalogHostError, SubnetCatalogLoadRequest, load_cached_subnet_catalog,
    load_subnet_catalog,
};
use std::path::{Path, PathBuf};

const IC_QUERY_CACHE_DIRECTORY: &str = "ic-query";

/// Load Canic's validated mainnet Subnet Catalog under an explicit repair policy.
pub fn load_mainnet_subnet_catalog(
    icp_root: &Path,
    now_unix_secs: u64,
) -> Result<CatalogLoadOutcome, SubnetCatalogHostError> {
    let request = mainnet_subnet_catalog_load_request(icp_root, now_unix_secs);
    load_subnet_catalog(&request)
}

/// Load existing validated mainnet evidence without a network call or cache mutation.
pub fn load_cached_mainnet_subnet_catalog(
    icp_root: &Path,
    now_unix_secs: u64,
) -> Result<CatalogLoadOutcome, SubnetCatalogHostError> {
    let request = mainnet_subnet_catalog_cache_only_request(icp_root, now_unix_secs);
    load_cached_subnet_catalog(&request)
}

fn mainnet_subnet_catalog_load_request(
    icp_root: &Path,
    now_unix_secs: u64,
) -> SubnetCatalogLoadRequest {
    let cache = SubnetCatalogCacheRequest::new(
        mainnet_subnet_catalog_cache_root(icp_root),
        MAINNET_NETWORK,
    );
    let source = CatalogSourceSelection::uncertified_query(DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT);
    let request =
        SubnetCatalogLoadRequest::refresh_missing_or_invalid(cache, source, now_unix_secs);
    request.with_minimum_assurance(CatalogAssurance::UncertifiedQuery)
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
        .with_minimum_assurance(CatalogAssurance::UncertifiedQuery)
}

/// Return the private capability root used for Canic's embedded `ic-query` cache.
#[must_use]
pub fn mainnet_subnet_catalog_cache_root(icp_root: &Path) -> PathBuf {
    icp_root.join(".canic").join(IC_QUERY_CACHE_DIRECTORY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_query::subnet_catalog::CatalogReadPolicy;

    #[test]
    fn mainnet_load_request_freezes_source_and_minimum_assurance() {
        let request = mainnet_subnet_catalog_load_request(Path::new("/tmp/canic-test"), 123);

        assert_eq!(
            request.minimum_assurance,
            CatalogAssurance::UncertifiedQuery
        );
        assert_eq!(request.now_unix_secs, 123);
        assert_eq!(request.cache.network, MAINNET_NETWORK);
        assert_eq!(
            request.cache.cache_root,
            Path::new("/tmp/canic-test/.canic/ic-query")
        );
        assert_eq!(
            request.policy,
            CatalogReadPolicy::RefreshMissingOrInvalid {
                source: CatalogSourceSelection::uncertified_query(
                    DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT,
                ),
            }
        );
    }

    #[test]
    fn mainnet_preflight_request_is_cache_only_and_read_only() {
        let request = mainnet_subnet_catalog_cache_only_request(Path::new("/tmp/canic-test"), 123);

        assert_eq!(
            request.minimum_assurance,
            CatalogAssurance::UncertifiedQuery
        );
        assert_eq!(request.now_unix_secs, 123);
        assert_eq!(request.cache.network, MAINNET_NETWORK);
        assert_eq!(
            request.cache.cache_root,
            Path::new("/tmp/canic-test/.canic/ic-query")
        );
        assert_eq!(request.policy, CatalogReadPolicy::CacheOnly);
    }
}
