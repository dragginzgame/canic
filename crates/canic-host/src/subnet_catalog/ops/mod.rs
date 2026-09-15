//! Module: subnet_catalog::ops
//!
//! Responsibility: project validated upstream acquisition evidence for Canic reports.
//! Boundary: no refresh, authority mutation or trust promotion occurs here.

use crate::subnet_catalog::{
    MAINNET_CATALOG_MAX_AGE_SECONDS,
    view::{RegistryCollectionProgress, RegistryCollectionStage, SubnetCatalogObservation},
};
use ic_query::subnet_catalog::{
    CatalogLoadOutcome, SubnetCatalogProgress, SubnetCatalogProgressPhase, catalog_stale_status,
};

/// Preserve stable snapshot facts separately from transient cache acquisition facts.
#[must_use]
pub fn observation(outcome: &CatalogLoadOutcome, now_unix_secs: u64) -> SubnetCatalogObservation {
    let authority = outcome.snapshot_authority();
    let freshness = catalog_stale_status(
        outcome.catalog.raw(),
        now_unix_secs,
        MAINNET_CATALOG_MAX_AGE_SECONDS,
    );
    SubnetCatalogObservation {
        cache_path: outcome.path.clone(),
        cache_disposition: outcome.disposition.as_str().to_string(),
        fetched_at: outcome.catalog.provenance().fetched_at.clone(),
        observed_at_unix_secs: now_unix_secs,
        age_seconds: freshness.age_seconds,
        max_age_seconds: MAINNET_CATALOG_MAX_AGE_SECONDS,
        registry_version: authority.registry_version,
        catalog_digest: authority.catalog_digest,
        assurance: authority.assurance.as_str().to_string(),
        source_endpoints: authority.source_endpoints,
    }
}

/// Preserve typed upstream progress without promoting partial collection to authority.
pub(super) fn registry_progress(event: SubnetCatalogProgress) -> RegistryCollectionProgress {
    let stage = match event.phase {
        SubnetCatalogProgressPhase::EndpointStarted => RegistryCollectionStage::Started,
        SubnetCatalogProgressPhase::Pinned { registry_version } => {
            RegistryCollectionStage::Pinned { registry_version }
        }
        SubnetCatalogProgressPhase::History {
            registry_version,
            through_version,
            reused,
        } => RegistryCollectionStage::History {
            registry_version,
            through_version,
            reused,
        },
        SubnetCatalogProgressPhase::Record {
            registry_version,
            key,
            completed,
        } => RegistryCollectionStage::Record {
            registry_version,
            key,
            completed,
        },
        SubnetCatalogProgressPhase::Retry {
            method,
            next_attempt,
            delay_millis,
        } => RegistryCollectionStage::Retry {
            method: method.to_string(),
            next_attempt,
            delay_millis,
        },
    };
    RegistryCollectionProgress {
        endpoint: event.endpoint,
        query_calls: event.query_call_count,
        stage,
    }
}
