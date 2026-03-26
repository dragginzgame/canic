use crate::dto::prelude::*;

///
/// Metrics DTOs
///
/// WHY THIS MODULE EXISTS
/// ----------------------
/// This module defines the **public, serialized representation** of metrics
/// exposed by the system.
///
/// These types are:
/// - Read-only snapshots
/// - Emitted at query/read time
/// - Detached from internal storage or aggregation strategy
///
/// Invariants:
/// - These structs MUST remain stable across upgrades.
/// - They MUST NOT encode internal metric layout or backend details.
/// - Cardinality must remain bounded by design.
///
/// Any change here affects:
/// - External dashboards
/// - Monitoring integrations
/// - Long-term metric continuity
///
/// Treat changes as **breaking API changes**.
///

///
/// MetricsKind
///
/// Metric family selector for the unified metrics query endpoint.
///

#[derive(CandidType, Clone, Copy, Deserialize)]
pub enum MetricsKind {
    System,
    Icc,
    Http,
    Timer,
    Access,
    Delegation,
    RootCapability,
    CyclesFunding,
    Perf,
}

///
/// MetricEntry
///
/// Unified metrics row for all query families.
///
/// The requested `MetricsKind` defines the meaning of `value`.
///

#[derive(CandidType, Deserialize)]
pub struct MetricEntry {
    /// Ordered, low-cardinality labels for the requested metric family.
    pub labels: Vec<String>,

    /// Optional principal dimension.
    pub principal: Option<Principal>,

    /// Metric payload for the requested family.
    pub value: MetricValue,
}

///
/// MetricValue
///

#[derive(CandidType, Deserialize)]
pub enum MetricValue {
    Count(u64),
    CountAndU64 { count: u64, value_u64: u64 },
    U128(u128),
}
