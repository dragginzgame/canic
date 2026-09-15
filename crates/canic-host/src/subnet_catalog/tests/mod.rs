mod acquisition;
mod source;

use super::*;
use crate::test_support::temp_dir;
use ic_query::subnet_catalog::{
    CacheDisposition, CatalogReadPolicy, SubnetCatalogHostError, SubnetCatalogSource,
    load_subnet_catalog_detailed_with_source,
};
use source::{Reply, Source};
use std::{fs, sync::atomic::Ordering, time::Duration};

const NOW: u64 = 1_780_531_200;

#[test]
fn freshness_boundary_refreshes_only_expired_catalog_and_preserves_snapshot_identity_on_hit() {
    let root = temp_dir("canic-catalog-expiry");
    let source = Source::new(Reply::Matching, 10);
    let mut request = mainnet_subnet_catalog_load_request(&root, NOW);
    let first = load_subnet_catalog_detailed_with_source(&request, &source).unwrap();
    assert_eq!(first.disposition, CacheDisposition::RefreshedMissing);
    assert_eq!(
        source.calls.load(Ordering::Relaxed),
        MAINNET_CATALOG_ENDPOINTS.len()
    );
    assert_eq!(
        first.catalog.provenance().assurance,
        CatalogAssurance::MultiEndpointAgreement
    );
    request.now_unix_secs += MAINNET_CATALOG_MAX_AGE_SECONDS;
    let hit = load_subnet_catalog_detailed_with_source(&request, &source).unwrap();
    assert_eq!(hit.disposition, CacheDisposition::CacheHit);
    assert_eq!(first.snapshot_authority(), hit.snapshot_authority());
    assert_eq!(
        source.calls.load(Ordering::Relaxed),
        MAINNET_CATALOG_ENDPOINTS.len()
    );
    let report = ops::observation(&hit, request.now_unix_secs);
    assert_eq!(report.age_seconds, Some(MAINNET_CATALOG_MAX_AGE_SECONDS));
    assert_eq!(report.cache_disposition, "cache_hit");
    assert_eq!(
        report.catalog_digest,
        first.snapshot_authority().catalog_digest
    );
    request.now_unix_secs += 1;
    let refreshed =
        load_subnet_catalog_detailed_with_source(&request, &Source::new(Reply::Matching, 11))
            .unwrap();
    assert_eq!(refreshed.disposition, CacheDisposition::RefreshedStale);
    assert_ne!(first.snapshot_authority(), refreshed.snapshot_authority());
    assert_eq!(first.snapshot_authority().registry_version, 10);
    assert_eq!(refreshed.snapshot_authority().registry_version, 11);
    assert_eq!(
        ops::observation(&refreshed, request.now_unix_secs).age_seconds,
        Some(0)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn agreement_failure_preserves_cache_and_retry_recovers_without_weaker_fallback() {
    for reply in [
        Reply::DifferentVersion,
        Reply::DifferentPayload,
        Reply::Unavailable,
    ] {
        let root = temp_dir("canic-catalog-agreement-failure");
        let mut request = mainnet_subnet_catalog_load_request(&root, NOW);
        let initial =
            load_subnet_catalog_detailed_with_source(&request, &Source::new(Reply::Matching, 10))
                .unwrap();
        let bytes = fs::read(&initial.path).unwrap();
        request.now_unix_secs += MAINNET_CATALOG_MAX_AGE_SECONDS + 1;
        let failure = load_subnet_catalog_detailed_with_source(&request, &Source::new(reply, 11))
            .unwrap_err();
        match reply {
            Reply::DifferentVersion | Reply::DifferentPayload => assert!(matches!(
                failure.source,
                SubnetCatalogHostError::AgreementMismatch { .. }
            )),
            Reply::Unavailable => assert!(matches!(
                failure.source,
                SubnetCatalogHostError::AgreementEndpoint { .. }
            )),
            Reply::Matching => unreachable!(),
        }
        assert_eq!(fs::read(&initial.path).unwrap(), bytes);
        let retained = load_cached_mainnet_subnet_catalog(&root, request.now_unix_secs).unwrap();
        assert_eq!(retained.snapshot_authority(), initial.snapshot_authority());
        assert_eq!(fs::read(&initial.path).unwrap(), bytes);
        let evidence = SubnetCatalogLoadFailureEvidenceV1::from_preflight_failure(&failure);
        assert_eq!(
            evidence.source_kind,
            Some(SubnetCatalogSourceKindV1::MultiEndpointAgreement)
        );
        assert_eq!(evidence.stage, SubnetCatalogLoadStageV1::RefreshFailed);
        assert_eq!(
            evidence.cache_disposition,
            SubnetCatalogFailureCacheDispositionV1::RefreshFailed {
                trigger: SubnetCatalogRefreshTriggerV1::Stale
            }
        );
        let recovered =
            load_subnet_catalog_detailed_with_source(&request, &Source::new(Reply::Matching, 11))
                .unwrap();
        assert_eq!(recovered.disposition, CacheDisposition::RefreshedStale);
        assert_eq!(recovered.snapshot_authority().registry_version, 11);
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn insufficient_assurance_cache_is_rejected_and_new_generation_refreshes_it() {
    let root = temp_dir("canic-catalog-assurance");
    let weak = SubnetCatalogLoadRequest::refresh_missing_or_invalid(
        mainnet_subnet_catalog_load_request(&root, NOW).cache,
        CatalogSourceSelection::uncertified_query(MAINNET_CATALOG_ENDPOINTS[0]),
        NOW,
    );
    let initial =
        load_subnet_catalog_detailed_with_source(&weak, &Source::new(Reply::Matching, 10)).unwrap();
    let bytes = fs::read(&initial.path).unwrap();
    let failure = load_cached_mainnet_subnet_catalog(&root, NOW).unwrap_err();
    assert_eq!(
        failure.stage,
        ic_query::subnet_catalog::SubnetCatalogLoadStage::CacheRejection
    );
    assert_eq!(fs::read(&initial.path).unwrap(), bytes);
    let failure = load_mainnet_with_source(&root, NOW, &Source::new(Reply::Unavailable, 10), None)
        .unwrap_err();
    assert!(matches!(
        *failure,
        crate::subnet_catalog::acquisition::CatalogAcquisitionError::Load(failure) if matches!(failure.source, SubnetCatalogHostError::AgreementEndpoint { .. })
    ));
    assert_eq!(fs::read(&initial.path).unwrap(), bytes);
    let upgraded =
        load_mainnet_with_source(&root, NOW, &Source::new(Reply::Matching, 10), None).unwrap();
    assert_eq!(upgraded.disposition, CacheDisposition::ForcedRefresh);
    assert_eq!(
        upgraded.catalog.provenance().assurance,
        CatalogAssurance::MultiEndpointAgreement
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn mainnet_load_request_freezes_source_and_minimum_assurance() {
    let request = mainnet_subnet_catalog_load_request(Path::new("/tmp/canic-test"), 123);

    assert_eq!(
        request.minimum_assurance,
        CatalogAssurance::MultiEndpointAgreement
    );
    assert_eq!(request.now_unix_secs, 123);
    assert_eq!(request.cache.network, MAINNET_NETWORK);
    assert_eq!(
        request.cache.cache_root,
        Path::new("/tmp/canic-test/.canic/ic-query")
    );
    assert_eq!(
        request.policy,
        CatalogReadPolicy::RefreshMissingInvalidOrOlderThan {
            source: CatalogSourceSelection::multi_endpoint_agreement(
                MAINNET_CATALOG_ENDPOINTS
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect(),
            ),
            max_age_seconds: MAINNET_CATALOG_MAX_AGE_SECONDS,
        }
    );
}

#[test]
fn mainnet_preflight_request_is_cache_only_and_read_only() {
    let request = mainnet_subnet_catalog_cache_only_request(Path::new("/tmp/canic-test"), 123);

    assert_eq!(
        request.minimum_assurance,
        CatalogAssurance::MultiEndpointAgreement
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

    let failure =
        load_cached_mainnet_subnet_catalog(&root, 123).expect_err("missing cache must fail closed");
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

fn load_mainnet_with_source(
    icp_root: &Path,
    now_unix_secs: u64,
    source: &dyn SubnetCatalogSource,
    progress: Option<&(dyn Fn(&view::CatalogAcquisitionProgress) + Sync)>,
) -> Result<CatalogLoadOutcome, Box<crate::subnet_catalog::acquisition::CatalogAcquisitionError>> {
    let request = mainnet_subnet_catalog_load_request(icp_root, now_unix_secs);
    crate::subnet_catalog::acquisition::load(
        &request,
        source,
        progress,
        Duration::from_secs(MAINNET_CATALOG_ACQUISITION_SECONDS),
    )
}
