//! Bounded local RPC outcomes; authoritative receipts decide exact replay.
use canic::api::metrics::providers::{MetricDefinition, MetricKind, MetricProviderApi};

const COUNTERS: &[MetricDefinition] = &[
    MetricDefinition {
        kind: MetricKind::Counter,
        name: "admission.attempts",
        unit: "events",
    },
    MetricDefinition {
        kind: MetricKind::Counter,
        name: "admission.applied",
        unit: "events",
    },
    MetricDefinition {
        kind: MetricKind::Counter,
        name: "admission.replayed",
        unit: "events",
    },
    MetricDefinition {
        kind: MetricKind::Counter,
        name: "admission.refused",
        unit: "events",
    },
    MetricDefinition {
        kind: MetricKind::Measurement,
        name: "admission.instructions",
        unit: "instructions",
    },
    MetricDefinition {
        kind: MetricKind::Counter,
        name: "admission.request_bytes",
        unit: "bytes",
    },
    MetricDefinition {
        kind: MetricKind::Counter,
        name: "reference.attempts",
        unit: "events",
    },
    MetricDefinition {
        kind: MetricKind::Counter,
        name: "reference.applied",
        unit: "events",
    },
    MetricDefinition {
        kind: MetricKind::Counter,
        name: "reference.replayed",
        unit: "events",
    },
    MetricDefinition {
        kind: MetricKind::Counter,
        name: "reference.refused",
        unit: "events",
    },
    MetricDefinition {
        kind: MetricKind::Measurement,
        name: "reference.instructions",
        unit: "instructions",
    },
    MetricDefinition {
        kind: MetricKind::Counter,
        name: "reference.request_bytes",
        unit: "bytes",
    },
];
/// Register declarations before the sole Canic lifecycle activates its providers.
///
/// # Panics
/// Traps installation if the host repeats or cannot admit the fixed roster.
pub fn register() {
    MetricProviderApi::register("blob", COUNTERS).expect("Blob metric declarations");
}
