use crate::workflow::runtime::template::publication::WasmStorePublicationWorkflow;
use canic_core::__control_plane_core as cp_core;
use canic_core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
    WasmStoreMetricsApi,
};
use cp_core::InternalError;

impl WasmStorePublicationWorkflow {
    // Return true when one failed store call represents store-capacity exhaustion.
    pub(super) fn is_store_capacity_exceeded(err: &InternalError) -> bool {
        err.public_error().is_some_and(|public| {
            public
                .message
                .contains(Self::WASM_STORE_CAPACITY_EXCEEDED_MESSAGE)
        }) || err
            .to_string()
            .contains(Self::WASM_STORE_CAPACITY_EXCEEDED_MESSAGE)
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
        if WasmStorePublicationWorkflow::is_store_capacity_exceeded(err) {
            Self::Capacity
        } else if err.public_error().is_some() {
            Self::StoreCall
        } else if err.to_string().contains("chunk") {
            Self::MissingChunk
        } else {
            Self::InvalidState
        }
    }
}
