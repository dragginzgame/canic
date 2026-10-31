//! Module: runtime_probe::process_fixture
//!
//! Responsibility: stimulate the public Wasm-store recorder for cache qualification.
//! Does not own: platform calls, application data, or a sampling schedule.
//! Boundary: recorder instructions are separate from complete production-sampler work.

use canic::__internal::core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
    WasmStoreMetricsApi,
};

pub fn record() -> u64 {
    let start = ic_platform::cdk::api::performance_counter(0);
    WasmStoreMetricsApi::record(
        WasmStoreMetricOperation::ChunkUpload,
        WasmStoreMetricSource::TargetStore,
        WasmStoreMetricOutcome::Completed,
        WasmStoreMetricReason::Ok,
    );
    ic_platform::cdk::api::performance_counter(0).saturating_sub(start)
}
