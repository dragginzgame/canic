use crate::dto::{
    page::{Page, PageRequest},
    prelude::*,
};

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

/// MetricsKind
///
/// Metric family selector for the unified metrics query endpoint.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
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
/// MetricsRequest
///
/// Unified metrics query request envelope.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct MetricsRequest {
    pub kind: MetricsKind,
    pub page: PageRequest,
}

///
/// MetricsResponse
///
/// Unified metrics query response envelope.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct MetricsResponse {
    pub entries: Page<MetricEntry>,
}

///
/// MetricEntry
///
/// Unified metrics row for all query families.
///
/// The requested `MetricsKind` defines the meaning of the populated fields.
/// Empty fields are intentionally omitted as `None` to keep one stable
/// transport shape instead of many per-family DTO variants.
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct MetricEntry {
    /// Ordered, low-cardinality labels for the requested metric family.
    pub labels: Vec<String>,

    /// Optional principal dimension.
    pub principal: Option<Principal>,

    /// Optional count or event total.
    pub count: Option<u64>,

    /// Optional u64 value such as delay or total instructions.
    pub value_u64: Option<u64>,

    /// Optional u128 value such as cycles totals.
    pub value_u128: Option<u128>,
}
