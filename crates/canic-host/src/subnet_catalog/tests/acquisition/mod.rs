//! Acquisition cancellation and progress proofs using supplied, effect-free sources.

use crate::subnet_catalog::{
    MAINNET_CATALOG_ENDPOINTS, MAINNET_CATALOG_MAX_AGE_SECONDS,
    acquisition::{self, CatalogAcquisitionError},
    mainnet_subnet_catalog_load_request,
    tests::{
        NOW,
        source::{Reply, Source},
    },
    view::CatalogAcquisitionStage,
};
use crate::test_support::temp_dir;
use ic_query::{
    nns::NnsSourceRequest,
    subnet_catalog::{
        SubnetCatalogSource, SubnetCatalogSourceFuture, load_subnet_catalog_detailed_with_source,
        subnet_catalog_refresh_lock_path,
    },
};
use std::{
    fs,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

struct PendingSource {
    inner: Source,
    dropped: AtomicBool,
}

struct PendingGuard<'a>(&'a AtomicBool);

impl Drop for PendingGuard<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl SubnetCatalogSource for PendingSource {
    fn fetch_catalog<'a>(&'a self, request: &'a NnsSourceRequest) -> SubnetCatalogSourceFuture<'a> {
        Box::pin(async move {
            if request.endpoint == MAINNET_CATALOG_ENDPOINTS[1] {
                let _guard = PendingGuard(&self.dropped);
                std::future::pending::<()>().await;
            }
            self.inner.fetch_catalog(request).await
        })
    }
}

#[test]
fn timeout_drops_collection_preserves_cache_releases_lock_and_allows_exact_retry() {
    let root = temp_dir("canic-catalog-timeout");
    let mut request = mainnet_subnet_catalog_load_request(&root, NOW);
    let first =
        load_subnet_catalog_detailed_with_source(&request, &Source::new(Reply::Matching, 10))
            .unwrap();
    let bytes = fs::read(&first.path).unwrap();
    request.now_unix_secs += MAINNET_CATALOG_MAX_AGE_SECONDS + 1;
    let source = PendingSource {
        inner: Source::new(Reply::Matching, 11),
        dropped: AtomicBool::new(false),
    };
    let events = Mutex::new(Vec::new());
    let progress = |event: &crate::subnet_catalog::view::CatalogAcquisitionProgress| {
        events.lock().unwrap().push(event.clone());
    };
    let failure = acquisition::load(
        &request,
        &source,
        Some(&progress),
        Duration::from_millis(100),
    )
    .unwrap_err();
    assert!(
        matches!(*failure, CatalogAcquisitionError::Timeout(ref progress)
        if progress.completed_endpoints == 1 && matches!(&progress.stage, CatalogAcquisitionStage::Collecting { endpoint } if endpoint == MAINNET_CATALOG_ENDPOINTS[1]))
    );
    assert!(source.dropped.load(Ordering::SeqCst));
    assert_eq!(fs::read(&first.path).unwrap(), bytes);
    assert!(
        !subnet_catalog_refresh_lock_path(&request.cache.cache_root, &request.cache.network)
            .exists()
    );
    let events = events.into_inner().unwrap();
    assert!(matches!(
        events[0].stage,
        CatalogAcquisitionStage::CacheLookup
    ));
    assert!(events.iter().any(|event| matches!(
        event.stage,
        CatalogAcquisitionStage::Collected {
            registry_version: 11,
            ..
        }
    )));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event.stage, CatalogAcquisitionStage::Complete { .. }))
    );
    let recovered = acquisition::load(
        &request,
        &Source::new(Reply::Matching, 11),
        None,
        Duration::from_secs(2),
    )
    .unwrap();
    assert_eq!(recovered.snapshot_authority().registry_version, 11);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cache_hit_reports_completion_without_source_calls_inside_an_existing_runtime() {
    let root = temp_dir("canic-catalog-progress-hit");
    let request = mainnet_subnet_catalog_load_request(&root, NOW);
    let first =
        load_subnet_catalog_detailed_with_source(&request, &Source::new(Reply::Matching, 10))
            .unwrap();
    let source = PendingSource {
        inner: Source::new(Reply::Matching, 11),
        dropped: AtomicBool::new(false),
    };
    let events = Mutex::new(Vec::new());
    let progress = |event: &crate::subnet_catalog::view::CatalogAcquisitionProgress| {
        events.lock().unwrap().push(event.clone());
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let hit = runtime.block_on(async {
        acquisition::load(&request, &source, Some(&progress), Duration::from_secs(2)).unwrap()
    });
    assert_eq!(source.inner.calls.load(Ordering::Relaxed), 0);
    assert_eq!(hit.snapshot_authority(), first.snapshot_authority());
    assert!(
        matches!(events.lock().unwrap().last().unwrap().stage, CatalogAcquisitionStage::Complete { ref cache_disposition } if cache_disposition == "cache_hit")
    );
    fs::remove_dir_all(root).unwrap();
}

struct ConcurrentPendingSource {
    started: std::sync::atomic::AtomicUsize,
    dropped: std::sync::atomic::AtomicUsize,
}

struct CountDrop<'a>(&'a std::sync::atomic::AtomicUsize);

impl Drop for CountDrop<'_> {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

impl SubnetCatalogSource for ConcurrentPendingSource {
    fn fetch_catalog<'a>(&'a self, _: &'a NnsSourceRequest) -> SubnetCatalogSourceFuture<'a> {
        Box::pin(async move {
            let _guard = CountDrop(&self.dropped);
            self.started.fetch_add(1, Ordering::SeqCst);
            std::future::pending().await
        })
    }
}

#[test]
fn agreement_polls_both_endpoints_and_timeout_cancels_both() {
    let root = temp_dir("canic-concurrent-catalog");
    let request = mainnet_subnet_catalog_load_request(&root, NOW);
    let source = ConcurrentPendingSource {
        started: std::sync::atomic::AtomicUsize::new(0),
        dropped: std::sync::atomic::AtomicUsize::new(0),
    };
    let event = crate::subnet_catalog::ops::registry_progress(
        ic_query::subnet_catalog::SubnetCatalogProgress {
            endpoint: MAINNET_CATALOG_ENDPOINTS[0].to_string(),
            query_call_count: 20,
            phase: ic_query::subnet_catalog::SubnetCatalogProgressPhase::History {
                registry_version: 99,
                through_version: 80,
                reused: true,
            },
        },
    );
    let registry = Mutex::new(std::collections::BTreeMap::from([(
        event.endpoint.clone(),
        event.clone(),
    )]));
    let failure = acquisition::load_observed(
        &request,
        &source,
        None,
        Duration::from_millis(100),
        &registry,
    )
    .unwrap_err();
    let CatalogAcquisitionError::Timeout(progress) = *failure else {
        panic!("expected typed timeout")
    };
    assert_eq!(progress.active_endpoints, MAINNET_CATALOG_ENDPOINTS);
    assert_eq!(progress.registry, vec![event]);
    assert!(matches!(
        progress.registry[0].stage,
        crate::subnet_catalog::view::RegistryCollectionStage::History {
            registry_version: 99,
            through_version: 80,
            reused: true
        }
    ));
    assert_eq!(progress.registry[0].query_calls, 20);
    assert_eq!(progress.completed_endpoints, 0);
    assert_eq!(
        source.started.load(Ordering::SeqCst),
        MAINNET_CATALOG_ENDPOINTS.len()
    );
    assert_eq!(
        source.dropped.load(Ordering::SeqCst),
        MAINNET_CATALOG_ENDPOINTS.len()
    );
    assert!(
        !subnet_catalog_refresh_lock_path(&request.cache.cache_root, &request.cache.network)
            .exists()
    );
    fs::remove_dir_all(root).unwrap();
}
