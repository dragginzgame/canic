//! Module: subnet_catalog::acquisition::client
//!
//! Responsibility: retain one caller-owned live Registry source across acquisitions.
//! Boundary: history prefixes remain endpoint-local memory, never published authority.

use super::{CatalogAcquisitionError, load_observed};
use crate::subnet_catalog::{
    MAINNET_CATALOG_ACQUISITION_SECONDS, mainnet_subnet_catalog_load_request,
    ops::registry_progress,
    view::{CatalogAcquisitionProgress, RegistryCollectionProgress},
};
use ic_query::subnet_catalog::{CatalogLoadOutcome, LiveSubnetCatalogSource};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

///
/// MainnetCatalogClient
///
/// Host-owned acquisition client. Retain it across attempts to reuse validated
/// upstream history prefixes; each load still validates current snapshot authority.
///

pub struct MainnetCatalogClient {
    source: LiveSubnetCatalogSource,
    registry: Arc<Mutex<BTreeMap<String, RegistryCollectionProgress>>>,
}

impl Default for MainnetCatalogClient {
    fn default() -> Self {
        let registry = Arc::new(Mutex::new(BTreeMap::new()));
        let sink = Arc::clone(&registry);
        let source = LiveSubnetCatalogSource::with_progress(move |event| {
            sink.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(event.endpoint.clone(), registry_progress(event));
        });
        Self { source, registry }
    }
}

impl MainnetCatalogClient {
    /// Acquire current agreement evidence within Canic's shared deadline.
    /// Progress callbacks run on the joined worker and must return promptly.
    /// Exclusive access keeps transient progress separate across attempts.
    pub fn load(
        &mut self,
        root: &Path,
        now_unix_secs: u64,
        progress: Option<&(dyn Fn(&CatalogAcquisitionProgress) + Sync)>,
    ) -> Result<CatalogLoadOutcome, Box<CatalogAcquisitionError>> {
        self.registry
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        load_observed(
            &mainnet_subnet_catalog_load_request(root, now_unix_secs),
            &self.source,
            progress,
            Duration::from_secs(MAINNET_CATALOG_ACQUISITION_SECONDS),
            &self.registry,
        )
    }
}
