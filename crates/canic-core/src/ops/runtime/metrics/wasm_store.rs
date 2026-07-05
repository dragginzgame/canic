//! Module: ops::runtime::metrics::wasm_store
//!
//! Responsibility: record and snapshot low-cardinality runtime metrics for the wasm_store family.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

use std::{cell::RefCell, collections::HashMap};

pub use crate::domain::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
};

thread_local! {
    static WASM_STORE_METRICS: RefCell<HashMap<WasmStoreMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// WasmStoreMetricKey
///
/// Composite key for one low-cardinality wasm-store counter.
///

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct WasmStoreMetricKey {
    pub operation: WasmStoreMetricOperation,
    pub source: WasmStoreMetricSource,
    pub outcome: WasmStoreMetricOutcome,
    pub reason: WasmStoreMetricReason,
}

///
/// WasmStoreMetrics
///
/// Operations-layer recorder for wasm-store workflow counters.
///

pub struct WasmStoreMetrics;

impl WasmStoreMetrics {
    /// Record one wasm-store operation event.
    pub fn record(
        operation: WasmStoreMetricOperation,
        source: WasmStoreMetricSource,
        outcome: WasmStoreMetricOutcome,
        reason: WasmStoreMetricReason,
    ) {
        WASM_STORE_METRICS.with_borrow_mut(|counts| {
            let key = WasmStoreMetricKey {
                operation,
                source,
                outcome,
                reason,
            };
            let entry = counts.entry(key).or_insert(0);
            *entry = entry.saturating_add(1);
        });
    }

    /// Snapshot the current wasm-store metric table as stable rows.
    #[must_use]
    pub fn snapshot() -> Vec<(WasmStoreMetricKey, u64)> {
        WASM_STORE_METRICS
            .with_borrow(std::clone::Clone::clone)
            .into_iter()
            .collect()
    }

    /// Test-only helper: clear all wasm-store metrics.
    #[cfg(test)]
    pub fn reset() {
        WASM_STORE_METRICS.with_borrow_mut(HashMap::clear);
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Convert snapshots into a map for concise count assertions.
    fn snapshot_map() -> HashMap<WasmStoreMetricKey, u64> {
        WasmStoreMetrics::snapshot().into_iter().collect()
    }

    // Verify wasm-store metrics accumulate by operation, source, outcome, and reason.
    #[test]
    fn wasm_store_metrics_accumulate_by_operation_source_outcome_and_reason() {
        WasmStoreMetrics::reset();

        WasmStoreMetrics::record(
            WasmStoreMetricOperation::SourceResolve,
            WasmStoreMetricSource::Bootstrap,
            WasmStoreMetricOutcome::Completed,
            WasmStoreMetricReason::Ok,
        );
        WasmStoreMetrics::record(
            WasmStoreMetricOperation::ChunkUpload,
            WasmStoreMetricSource::Bootstrap,
            WasmStoreMetricOutcome::Skipped,
            WasmStoreMetricReason::CacheHit,
        );
        WasmStoreMetrics::record(
            WasmStoreMetricOperation::ChunkUpload,
            WasmStoreMetricSource::Bootstrap,
            WasmStoreMetricOutcome::Skipped,
            WasmStoreMetricReason::CacheHit,
        );

        let map = snapshot_map();

        assert_eq!(
            map.get(&WasmStoreMetricKey {
                operation: WasmStoreMetricOperation::SourceResolve,
                source: WasmStoreMetricSource::Bootstrap,
                outcome: WasmStoreMetricOutcome::Completed,
                reason: WasmStoreMetricReason::Ok,
            }),
            Some(&1)
        );
        assert_eq!(
            map.get(&WasmStoreMetricKey {
                operation: WasmStoreMetricOperation::ChunkUpload,
                source: WasmStoreMetricSource::Bootstrap,
                outcome: WasmStoreMetricOutcome::Skipped,
                reason: WasmStoreMetricReason::CacheHit,
            }),
            Some(&2)
        );
    }
}
