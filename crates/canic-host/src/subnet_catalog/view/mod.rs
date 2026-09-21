//! Module: subnet_catalog::view
//!
//! Responsibility: describe one validated catalog acquisition for operator review.
//! Boundary: diagnostic observations are not desired-state or operation authority.

use serde::Serialize;
use std::path::PathBuf;

///
/// CatalogAcquisitionStage
///
/// Current host acquisition phase; collecting an endpoint never implies agreement.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum CatalogAcquisitionStage {
    CacheLookup,
    Collecting {
        endpoint: String,
    },
    Collected {
        endpoint: String,
        registry_version: u64,
        query_calls: u64,
        /// Endpoint collection including certification; not pure remote wait.
        elapsed_micros: u128,
    },
    Complete {
        cache_disposition: String,
    },
}

///
/// CatalogAcquisitionProgress
///
/// Read-only progress at the acquisition boundary, including its whole-operation budget.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CatalogAcquisitionProgress {
    pub stage: CatalogAcquisitionStage,
    pub completed_endpoints: usize,
    pub active_endpoints: Vec<String>,
    pub registry: Vec<RegistryCollectionProgress>,
    pub elapsed_seconds: u64,
    pub elapsed_micros: u128,
    pub deadline_seconds: u64,
}

///
/// SubnetCatalogObservation
///
/// Read-only host projection of snapshot identity and transient acquisition facts.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SubnetCatalogObservation {
    pub cache_path: PathBuf,
    pub cache_disposition: String,
    pub fetched_at: String,
    pub observed_at_unix_secs: u64,
    pub age_seconds: Option<u64>,
    pub max_age_seconds: u64,
    pub registry_version: u64,
    pub catalog_digest: String,
    pub assurance: String,
    pub source_endpoints: Vec<String>,
}

///
/// RegistryCollectionProgress
///
/// Host diagnostic projection of the latest event for one endpoint.
/// Query attempts exclude upstream transport-internal retries and verification calls.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RegistryCollectionProgress {
    pub endpoint: String,
    pub query_calls: u64,
    pub stage: RegistryCollectionStage,
}

///
/// RegistryCollectionStage
///
/// Typed Registry acquisition detail for host progress and timeout reports.
/// Completed reads remain provisional until whole-catalog agreement validates.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum RegistryCollectionStage {
    Started,
    Pinned {
        registry_version: u64,
    },
    History {
        registry_version: u64,
        through_version: u64,
        reused: bool,
    },
    Record {
        registry_version: u64,
        key: String,
        completed: bool,
    },
    Retry {
        method: String,
        next_attempt: u8,
        delay_millis: u64,
    },
}
