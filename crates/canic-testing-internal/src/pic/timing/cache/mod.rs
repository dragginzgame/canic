//! Retain upstream cache phase timings without combining nested durations.

use super::{ACTIVE, emit};
use ic_testkit::artifacts::ArtifactCacheOutcome;
use serde::Serialize;

/// One completed cache acquisition, bound to its owning journey phase/process.
#[derive(Debug, Serialize)]
pub(super) struct CacheEvent {
    schema_version: u8,
    process_id: u32,
    worker: Option<u32>,
    parent_id: Option<u64>,
    cache: &'static str,
    reused: bool,
    coordination_lock_wait_us: u128,
    content_lock_wait_us: u128,
    namespace_lock_wait_us: u128,
    input_capture_us: u128,
    cache_lookup_us: u128,
    caller_build_us: Option<u128>,
    output_validation_us: u128,
    publication_us: u128,
    materialization_us: u128,
    maintenance_us: Option<u128>,
    total_us: u128,
}

impl CacheEvent {
    pub(super) fn new(cache: &'static str, outcome: &ArtifactCacheOutcome) -> Self {
        let timing = outcome.record().timings();
        Self {
            schema_version: 1,
            process_id: std::process::id(),
            worker: std::env::var("CANIC_POCKETIC_WORKER")
                .ok()
                .and_then(|value| value.parse().ok()),
            parent_id: ACTIVE.with_borrow(|active| active.last().copied()),
            cache,
            reused: outcome.is_reused(),
            coordination_lock_wait_us: timing.coordination_lock_wait().as_micros(),
            content_lock_wait_us: timing.content_lock_wait().as_micros(),
            namespace_lock_wait_us: timing.namespace_lock_wait().as_micros(),
            input_capture_us: timing.input_capture().as_micros(),
            cache_lookup_us: timing.cache_lookup().as_micros(),
            caller_build_us: timing.caller_build().map(|value| value.as_micros()),
            output_validation_us: timing.output_validation().as_micros(),
            publication_us: timing.publication().as_micros(),
            materialization_us: timing.materialization().as_micros(),
            maintenance_us: timing.maintenance().map(|value| value.as_micros()),
            total_us: timing.total().as_micros(),
        }
    }
}

pub(in crate::pic) fn artifact_cache(cache: &'static str, outcome: &ArtifactCacheOutcome) {
    emit("CANIC-CACHE", &CacheEvent::new(cache, outcome));
}
