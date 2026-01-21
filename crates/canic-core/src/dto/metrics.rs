use crate::{dto::prelude::*, ids::AccessMetricKind};

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
/// AccessMetricEntry
///
/// Snapshot entry pairing an endpoint with an access denial kind.
///
/// Access metrics are emitted only on denial and represent the kind and
/// predicate where access failed.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct AccessMetricEntry {
    /// Normalized endpoint name.
    ///
    /// This value originates from the access-layer metrics façade
    /// and must not include dynamic or user-derived data.
    pub endpoint: String,

    /// Access denial kind (guard, auth, env, rule, custom).
    pub kind: AccessMetricKind,

    /// Predicate name that denied access.
    ///
    /// This is either a built-in predicate name (e.g. "caller_is_root")
    /// or a custom predicate name returned by user-defined access checks.
    pub predicate: String,

    /// Total count for this (endpoint, kind, predicate) tuple.
    pub count: u64,
}

///
/// DelegationMetricEntry
///
/// Snapshot entry pairing a delegation authority with its usage count.
///
/// WHY THIS EXISTS:
/// - Delegated authorization introduces multiple signing authorities.
/// - This metric provides visibility into which authorities are active.
/// - Cardinality is bounded by the number of configured delegation certs.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct DelegationMetricEntry {
    /// Principal of the delegation authority (cert signer).
    pub authority: Principal,

    /// Number of successfully verified tokens attributed to this authority.
    pub count: u64,
}

///
/// EndpointAttemptMetricEntry
///
/// Snapshot entry for endpoint execution lifecycle metrics.
///
/// Semantics:
/// - `attempted` counts total execution attempts
/// - `completed` counts executions that reached completion
///
/// This metric does NOT encode success or failure.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct EndpointAttemptMetricEntry {
    /// Normalized endpoint name.
    pub endpoint: String,

    /// Number of times the endpoint was attempted.
    pub attempted: u64,

    /// Number of times execution completed.
    pub completed: u64,
}

///
/// EndpointResultMetricEntry
///
/// Snapshot entry for endpoint execution outcomes.
///
/// Semantics:
/// - `ok` counts successful executions
/// - `err` counts failed executions
///
/// This metric intentionally excludes *error causes* to prevent
/// high-cardinality labels.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct EndpointResultMetricEntry {
    /// Normalized endpoint name.
    pub endpoint: String,

    /// Number of successful executions.
    pub ok: u64,

    /// Number of failed executions.
    pub err: u64,
}

///
/// EndpointHealth
///
/// Derived, read-only view combining multiple metric streams.
///
/// IMPORTANT:
/// -----------
/// This struct is NOT stored directly.
/// It is materialized at read time by joining:
/// - access metrics
/// - attempt metrics
/// - result metrics
///
/// It exists purely for observability convenience.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct EndpointHealth {
    /// Normalized endpoint name.
    pub endpoint: String,

    /// Total execution attempts.
    pub attempted: u64,

    /// Total access denials.
    pub denied: u64,

    /// Total completed executions.
    pub completed: u64,

    /// Successful executions.
    pub ok: u64,

    /// Failed executions.
    pub err: u64,
}

///
/// HttpMetricEntry
///
/// Snapshot entry for HTTP ingress metrics.
///
/// Semantics:
/// - `method` is the HTTP verb (GET, POST, etc.)
/// - `label` is a low-cardinality classification
///
/// Labels MUST be controlled and finite.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct HttpMetricEntry {
    /// HTTP method (e.g. GET, POST).
    pub method: String,

    /// Controlled, low-cardinality label.
    pub label: String,

    /// Total count for this (method, label) pair.
    pub count: u64,
}

///
/// IccMetricEntry
///
/// Inter-canister call (ICC) metric entry.
///
/// Tracks outbound calls made by this canister.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct IccMetricEntry {
    /// Target canister principal.
    pub target: Principal,

    /// Method name invoked on the target.
    pub method: String,

    /// Number of calls made.
    pub count: u64,
}

///
/// SystemMetricEntry
///
/// Snapshot entry for internal system-level metrics.
///
/// `kind` is intentionally a string to allow extension without
/// schema changes, but MUST remain low-cardinality.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct SystemMetricEntry {
    /// Metric kind identifier.
    pub kind: String,

    /// Count for this metric kind.
    pub count: u64,
}

///
/// TimerMetricEntry
///
/// Snapshot entry for timer-based execution metrics.
///
/// Used to observe scheduled or delayed execution behavior.
///
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct TimerMetricEntry {
    /// Timer mode (e.g. one-shot, interval).
    pub mode: String,

    /// Delay in milliseconds.
    pub delay_ms: u64,

    /// Controlled label describing timer purpose.
    pub label: String,

    /// Number of times this timer fired.
    pub count: u64,
}
