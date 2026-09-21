//! Module: subnet_catalog::acquisition
//!
//! Responsibility: bound host catalog acquisition and expose typed progress.
//! Boundary: upstream owns collection, agreement, cache validation and atomic publication.

mod client;

use crate::subnet_catalog::{
    mainnet_source_selection,
    view::{CatalogAcquisitionProgress, CatalogAcquisitionStage, RegistryCollectionProgress},
};
use ic_query::{
    nns::NnsSourceRequest,
    subnet_catalog::{
        CatalogLoadOutcome, CatalogReadPolicy, SubnetCatalogDetailedSourceFuture,
        SubnetCatalogHostError, SubnetCatalogLoadFailure, SubnetCatalogLoadRequest,
        SubnetCatalogSource, SubnetCatalogSourceFuture,
        load_subnet_catalog_detailed_with_source_async,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Mutex, PoisonError},
    time::{Duration, Instant},
};
use thiserror::Error;

pub use client::MainnetCatalogClient;

///
/// CatalogAcquisitionError
///
/// Typed Canic acquisition failure; timeout never substitutes weaker cached evidence.
///

#[derive(Debug, Error)]
pub enum CatalogAcquisitionError {
    #[error(transparent)]
    Load(#[from] Box<SubnetCatalogLoadFailure>),

    #[error("could not start the Subnet Catalog acquisition runtime: {0}")]
    Runtime(#[source] std::io::Error),

    #[error("Subnet Catalog acquisition exceeded {} seconds after completing {} endpoint(s); retry generation", .0.deadline_seconds, .0.completed_endpoints)]
    Timeout(CatalogAcquisitionProgress),

    #[error("Subnet Catalog acquisition worker panicked")]
    WorkerPanicked,
}

#[cfg(test)]
pub(super) fn load(
    request: &SubnetCatalogLoadRequest,
    source: &dyn SubnetCatalogSource,
    progress: Option<&(dyn Fn(&CatalogAcquisitionProgress) + Sync)>,
    budget: Duration,
) -> Result<CatalogLoadOutcome, Box<CatalogAcquisitionError>> {
    load_observed(
        request,
        source,
        progress,
        budget,
        &Mutex::new(BTreeMap::new()),
    )
}

pub(super) fn load_observed(
    request: &SubnetCatalogLoadRequest,
    source: &dyn SubnetCatalogSource,
    progress: Option<&(dyn Fn(&CatalogAcquisitionProgress) + Sync)>,
    budget: Duration,
    registry: &Mutex<BTreeMap<String, RegistryCollectionProgress>>,
) -> Result<CatalogLoadOutcome, Box<CatalogAcquisitionError>> {
    std::thread::scope(|scope| {
        let worker = std::thread::Builder::new()
            .name("canic-subnet-catalog".into())
            .spawn_scoped(scope, || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(CatalogAcquisitionError::Runtime)?;
                runtime.block_on(load_async(request, source, progress, budget, registry))
            })
            .map_err(|error| Box::new(CatalogAcquisitionError::Runtime(error)))?;
        worker
            .join()
            .map_err(|_| Box::new(CatalogAcquisitionError::WorkerPanicked))?
    })
}

async fn load_async(
    request: &SubnetCatalogLoadRequest,
    source: &dyn SubnetCatalogSource,
    progress: Option<&(dyn Fn(&CatalogAcquisitionProgress) + Sync)>,
    budget: Duration,
    registry: &Mutex<BTreeMap<String, RegistryCollectionProgress>>,
) -> Result<CatalogLoadOutcome, Box<CatalogAcquisitionError>> {
    let tracked = TrackedSource {
        registry,
        source,
        progress,
        started: Instant::now(),
        budget,
        state: Mutex::new(AcquisitionState {
            stage: CatalogAcquisitionStage::CacheLookup,
            completed: 0,
            active: BTreeSet::new(),
        }),
    };
    tracked.emit();
    let future = load_with_assurance_repair(request, &tracked);
    tokio::pin!(future);
    let deadline = tokio::time::sleep(budget);
    tokio::pin!(deadline);
    let mut heartbeat = tokio::time::interval(Duration::from_secs(10));
    heartbeat.tick().await;
    loop {
        tokio::select! {
            result = &mut future => {
                let outcome = result.map_err(CatalogAcquisitionError::Load)?;
                tracked.set_stage(CatalogAcquisitionStage::Complete {
                    cache_disposition: outcome.disposition.as_str().to_string(),
                });
                return Ok(outcome);
            }
            () = &mut deadline => {
                return Err(Box::new(CatalogAcquisitionError::Timeout(tracked.snapshot())));
            }
            _ = heartbeat.tick() => tracked.emit(),
        }
    }
}

async fn load_with_assurance_repair(
    request: &SubnetCatalogLoadRequest,
    source: &dyn SubnetCatalogSource,
) -> Result<CatalogLoadOutcome, Box<SubnetCatalogLoadFailure>> {
    match load_subnet_catalog_detailed_with_source_async(request, source).await {
        Err(failure)
            if matches!(
                failure.source,
                SubnetCatalogHostError::InsufficientAssurance { .. }
            ) =>
        {
            let refresh = request
                .clone()
                .with_policy(CatalogReadPolicy::ForceRefresh {
                    source: mainnet_source_selection(),
                });
            load_subnet_catalog_detailed_with_source_async(&refresh, source).await
        }
        result => result,
    }
}

struct AcquisitionState {
    stage: CatalogAcquisitionStage,
    completed: usize,
    active: BTreeSet<String>,
}

struct TrackedSource<'a> {
    registry: &'a Mutex<BTreeMap<String, RegistryCollectionProgress>>,
    source: &'a dyn SubnetCatalogSource,
    progress: Option<&'a (dyn Fn(&CatalogAcquisitionProgress) + Sync)>,
    started: Instant,
    budget: Duration,
    state: Mutex<AcquisitionState>,
}

impl TrackedSource<'_> {
    fn snapshot(&self) -> CatalogAcquisitionProgress {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        CatalogAcquisitionProgress {
            stage: state.stage.clone(),
            completed_endpoints: state.completed,
            active_endpoints: state.active.iter().cloned().collect(),
            registry: self
                .registry
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .values()
                .cloned()
                .collect(),
            elapsed_seconds: self.started.elapsed().as_secs(),
            elapsed_micros: self.started.elapsed().as_micros(),
            deadline_seconds: self.budget.as_secs(),
        }
    }

    fn emit(&self) {
        if let Some(progress) = self.progress {
            progress(&self.snapshot());
        }
    }

    fn set_stage(&self, stage: CatalogAcquisitionStage) {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .stage = stage;
        self.emit();
    }
}

impl SubnetCatalogSource for TrackedSource<'_> {
    fn fetch_catalog<'a>(&'a self, request: &'a NnsSourceRequest) -> SubnetCatalogSourceFuture<'a> {
        Box::pin(async move {
            self.fetch_catalog_detailed(request)
                .await
                .map_err(ic_query::subnet_catalog::SubnetCatalogSourceFailure::into_source)
        })
    }

    fn fetch_catalog_detailed<'a>(
        &'a self,
        request: &'a NnsSourceRequest,
    ) -> SubnetCatalogDetailedSourceFuture<'a> {
        Box::pin(async move {
            self.state
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .active
                .insert(request.endpoint.clone());
            self.set_stage(CatalogAcquisitionStage::Collecting {
                endpoint: request.endpoint.clone(),
            });
            let started = Instant::now();
            let result = self.source.fetch_catalog_detailed(request).await;
            self.state
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .active
                .remove(&request.endpoint);
            if let Ok(raw) = &result {
                self.state
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .completed += 1;
                self.set_stage(CatalogAcquisitionStage::Collected {
                    endpoint: request.endpoint.clone(),
                    registry_version: raw.provenance.registry_version,
                    query_calls: raw.provenance.registry_query_call_count,
                    elapsed_micros: started.elapsed().as_micros(),
                });
            }
            result
        })
    }
}
