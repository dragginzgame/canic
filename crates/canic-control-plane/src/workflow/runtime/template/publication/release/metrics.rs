use crate::workflow::runtime::template::record_wasm_store_metric;
use canic_core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
};
use canic_core::control_plane_support::error::InternalError;
use canic_core::diagnostics::codes;

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
        match err.public_error().code() {
            code if code == codes::CAPACITY_LIMIT.raw_code() => Self::Capacity,
            code if code == codes::WASM_STORE_CHUNK_MISSING.raw_code() => Self::MissingChunk,
            code if code == codes::DIGEST_CONFLICT.raw_code() => Self::HashMismatch,
            code if code == codes::WASM_STORE_MANIFEST_MISSING.raw_code() => Self::MissingManifest,
            code if code == codes::STATE_CONFLICT.raw_code()
                || code == codes::STATE_INVALID.raw_code()
                || code == codes::COLLECTION_UNAVAILABLE.raw_code() =>
            {
                Self::InvalidState
            }
            _ => Self::StoreCall,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_failure_reasons_use_public_codes() {
        let cases = [
            (codes::CAPACITY_LIMIT, WasmStoreMetricReason::Capacity),
            (
                codes::WASM_STORE_CHUNK_MISSING,
                WasmStoreMetricReason::MissingChunk,
            ),
            (codes::DIGEST_CONFLICT, WasmStoreMetricReason::HashMismatch),
            (
                codes::WASM_STORE_MANIFEST_MISSING,
                WasmStoreMetricReason::MissingManifest,
            ),
            (codes::STATE_CONFLICT, WasmStoreMetricReason::InvalidState),
            (codes::STATE_INVALID, WasmStoreMetricReason::InvalidState),
            (
                codes::COLLECTION_UNAVAILABLE,
                WasmStoreMetricReason::InvalidState,
            ),
            (codes::STATE_UNAVAILABLE, WasmStoreMetricReason::StoreCall),
        ];

        for (code, expected) in cases {
            let err = InternalError::public(code);
            assert!(WasmStoreMetricReason::from_publication_error(&err) == expected);
        }
    }
}
