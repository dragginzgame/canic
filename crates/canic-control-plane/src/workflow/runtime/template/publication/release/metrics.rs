use crate::workflow::runtime::template::publication::WasmStorePublicationWorkflow;
use canic_core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
    WasmStoreMetricsApi,
};
use canic_core::control_plane_support::error::InternalError;
use canic_core::dto::error::ErrorCode;

impl WasmStorePublicationWorkflow {
    // Return true when one failed store call represents store-capacity exhaustion.
    pub(super) fn is_store_capacity_exceeded(err: &InternalError) -> bool {
        err.public_error()
            .is_some_and(|public| public.code == ErrorCode::WasmStoreCapacityExceeded)
    }
}

// Record one wasm-store metric point through the core API facade.
pub(super) fn record_wasm_store_metric(
    operation: WasmStoreMetricOperation,
    source: WasmStoreMetricSource,
    outcome: WasmStoreMetricOutcome,
    reason: WasmStoreMetricReason,
) {
    WasmStoreMetricsApi::record(operation, source, outcome, reason);
}

// Record one target-store release publish failure reason.
pub(super) fn record_wasm_store_publish_failed(reason: WasmStoreMetricReason) {
    record_wasm_store_metric(
        WasmStoreMetricOperation::ReleasePublish,
        WasmStoreMetricSource::TargetStore,
        WasmStoreMetricOutcome::Failed,
        reason,
    );
}

// Map publication failures into stable wasm-store metric reasons.
pub(super) trait WasmStorePublicationError {
    fn from_publication_error(err: &InternalError) -> Self;
}

impl WasmStorePublicationError for WasmStoreMetricReason {
    fn from_publication_error(err: &InternalError) -> Self {
        match err.public_error().map(|public| public.code) {
            Some(ErrorCode::WasmStoreCapacityExceeded) => Self::Capacity,
            Some(ErrorCode::WasmStoreChunkMissing) => Self::MissingChunk,
            Some(ErrorCode::WasmStoreHashMismatch) => Self::HashMismatch,
            Some(ErrorCode::WasmStoreManifestMissing) => Self::MissingManifest,
            Some(_) => Self::StoreCall,
            None => Self::InvalidState,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::dto::error::Error;

    #[test]
    fn publication_failure_reasons_use_public_codes() {
        let cases = [
            (
                ErrorCode::WasmStoreCapacityExceeded,
                WasmStoreMetricReason::Capacity,
            ),
            (
                ErrorCode::WasmStoreChunkMissing,
                WasmStoreMetricReason::MissingChunk,
            ),
            (
                ErrorCode::WasmStoreHashMismatch,
                WasmStoreMetricReason::HashMismatch,
            ),
            (
                ErrorCode::WasmStoreManifestMissing,
                WasmStoreMetricReason::MissingManifest,
            ),
        ];

        for (code, expected) in cases {
            let err = InternalError::public(Error::new(code, "detail".to_string()));
            assert!(WasmStoreMetricReason::from_publication_error(&err) == expected);
        }
    }
}
