use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static WASM_STORE_METRICS: RefCell<HashMap<WasmStoreMetricKey, u64>> =
        RefCell::new(HashMap::new());
}

///
/// WasmStoreMetricOperation
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum WasmStoreMetricOperation {
    BootstrapChunkSync,
    ChunkPublish,
    ChunkUpload,
    ManifestPromote,
    Prepare,
    ReleasePublish,
    SourceResolve,
}

impl WasmStoreMetricOperation {
    /// Return the stable public metrics label for this operation.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::BootstrapChunkSync => "bootstrap_chunk_sync",
            Self::ChunkPublish => "chunk_publish",
            Self::ChunkUpload => "chunk_upload",
            Self::ManifestPromote => "manifest_promote",
            Self::Prepare => "prepare",
            Self::ReleasePublish => "release_publish",
            Self::SourceResolve => "source_resolve",
        }
    }
}

///
/// WasmStoreMetricSource
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum WasmStoreMetricSource {
    Bootstrap,
    Embedded,
    ManagedFleet,
    Resolver,
    Store,
    TargetStore,
}

impl WasmStoreMetricSource {
    /// Return the stable public metrics label for this source.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap",
            Self::Embedded => "embedded",
            Self::ManagedFleet => "managed_fleet",
            Self::Resolver => "resolver",
            Self::Store => "store",
            Self::TargetStore => "target_store",
        }
    }
}

///
/// WasmStoreMetricOutcome
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum WasmStoreMetricOutcome {
    Completed,
    Failed,
    Skipped,
    Started,
}

impl WasmStoreMetricOutcome {
    /// Return the stable public metrics label for this outcome.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Started => "started",
        }
    }
}

///
/// WasmStoreMetricReason
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum WasmStoreMetricReason {
    CacheHit,
    CacheMiss,
    Capacity,
    HashMismatch,
    InvalidState,
    ManagementCall,
    MissingChunk,
    MissingManifest,
    Ok,
    StoreCall,
    UnsupportedInline,
}

impl WasmStoreMetricReason {
    /// Return the stable public metrics label for this reason.
    #[must_use]
    pub const fn metric_label(self) -> &'static str {
        match self {
            Self::CacheHit => "cache_hit",
            Self::CacheMiss => "cache_miss",
            Self::Capacity => "capacity",
            Self::HashMismatch => "hash_mismatch",
            Self::InvalidState => "invalid_state",
            Self::ManagementCall => "management_call",
            Self::MissingChunk => "missing_chunk",
            Self::MissingManifest => "missing_manifest",
            Self::Ok => "ok",
            Self::StoreCall => "store_call",
            Self::UnsupportedInline => "unsupported_inline",
        }
    }
}

///
/// WasmStoreMetricKey
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

///
/// TESTS
///

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
