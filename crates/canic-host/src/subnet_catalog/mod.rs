//! Module: subnet_catalog
//!
//! Responsibility: bind Canic's host-only Subnet Catalog cache and refresh authority.
//! Does not own: Subnet classification, Registry collection, or placement policy.
//! Boundary: callers receive only the validated catalog produced by `ic-query`.

mod evidence;

pub use evidence::{
    SubnetCatalogFailureCacheDispositionV1, SubnetCatalogFailureEffectsV1, SubnetCatalogFieldV1,
    SubnetCatalogLoadFailureEvidenceV1, SubnetCatalogLoadStageV1, SubnetCatalogRefreshTriggerV1,
    SubnetCatalogRegistryRecordEvidenceV1, SubnetCatalogRegistryRecordKindV1,
    SubnetCatalogRegistryValueEncodingV1, SubnetCatalogRetryabilityV1, SubnetCatalogSourceKindV1,
    SubnetCatalogSubjectV1, SubnetCatalogUnknownRetryReasonV1,
};

use ic_query::subnet_catalog::{
    CatalogAssurance, CatalogLoadOutcome, CatalogSourceSelection,
    DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT, MAINNET_NETWORK, SubnetCatalogCacheRequest,
    SubnetCatalogLoadFailure, SubnetCatalogLoadRequest, load_cached_subnet_catalog_detailed,
    load_subnet_catalog_detailed,
};
use std::path::{Path, PathBuf};

const IC_QUERY_CACHE_DIRECTORY: &str = "ic-query";

/// Load Canic's validated mainnet Subnet Catalog under an explicit repair policy.
pub fn load_mainnet_subnet_catalog(
    icp_root: &Path,
    now_unix_secs: u64,
) -> Result<CatalogLoadOutcome, Box<SubnetCatalogLoadFailure>> {
    let request = mainnet_subnet_catalog_load_request(icp_root, now_unix_secs);
    load_subnet_catalog_detailed(&request).map_err(Box::new)
}

/// Load existing validated mainnet evidence without a network call or cache mutation.
pub fn load_cached_mainnet_subnet_catalog(
    icp_root: &Path,
    now_unix_secs: u64,
) -> Result<CatalogLoadOutcome, Box<SubnetCatalogLoadFailure>> {
    let request = mainnet_subnet_catalog_cache_only_request(icp_root, now_unix_secs);
    load_cached_subnet_catalog_detailed(&request).map_err(Box::new)
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
    use crate::test_support::temp_dir;
    use ic_query::subnet_catalog::CatalogReadPolicy;
    use std::fs;

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

    #[test]
    fn cache_failure_reaches_canic_as_complete_typed_pre_effect_evidence() {
        let root = temp_dir("canic-subnet-catalog-detailed-failure");
        fs::create_dir_all(&root).expect("create temporary ICP root");

        let failure = load_cached_mainnet_subnet_catalog(&root, 123)
            .expect_err("missing cache must fail closed");
        let evidence = SubnetCatalogLoadFailureEvidenceV1::from_preflight_failure(&failure);

        fs::remove_dir_all(root).expect("remove temporary ICP root");
        assert_eq!(evidence.network, MAINNET_NETWORK);
        assert_eq!(evidence.source_kind, None);
        assert!(evidence.source_endpoints.is_empty());
        assert_eq!(evidence.stage, SubnetCatalogLoadStageV1::CacheAbsence);
        assert_eq!(evidence.registry_version, None);
        assert_eq!(evidence.returned_registry_value_version, None);
        assert_eq!(evidence.source_endpoint, None);
        assert_eq!(evidence.assurance, None);
        assert!(evidence.registry_records.is_empty());
        assert_eq!(
            evidence.cache_disposition,
            SubnetCatalogFailureCacheDispositionV1::CacheMissing
        );
        assert!(matches!(
            evidence.subject,
            Some(SubnetCatalogSubjectV1::CachePath { .. })
        ));
        assert_eq!(evidence.code, "missing_catalog");
        assert_eq!(evidence.category, "missing");
        assert_eq!(
            evidence.retryability,
            SubnetCatalogRetryabilityV1::NotRetryable
        );
        assert!(!evidence.effects.build_started);
        assert!(!evidence.effects.workspace_mutation_started);
        assert!(!evidence.effects.ic_mutation_started);
    }
}
