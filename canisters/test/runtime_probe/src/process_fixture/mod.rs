//! Module: runtime_probe::process_fixture
//!
//! Responsibility: stimulate the public Wasm-store recorder for cache qualification.
//! Does not own: platform calls, application data, or a sampling schedule.
//! Boundary: synthetic instrumentation precedes measurement of the complete production sampler.

use canic::__internal::core::api::lifecycle::metrics::{
    WasmStoreMetricOperation, WasmStoreMetricOutcome, WasmStoreMetricReason, WasmStoreMetricSource,
    WasmStoreMetricsApi,
};

pub fn record() {
    WasmStoreMetricsApi::record(
        WasmStoreMetricOperation::SourceResolve,
        WasmStoreMetricSource::Bootstrap,
        WasmStoreMetricOutcome::Completed,
        WasmStoreMetricReason::Ok,
    );
}
