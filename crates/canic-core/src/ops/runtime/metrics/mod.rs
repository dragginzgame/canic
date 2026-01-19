pub mod access;
pub mod delegation;
pub mod endpoint;
pub mod http;
pub mod icc;
pub mod mapper;
pub mod system;
pub mod timer;

use {
    access::{AccessMetrics, AccessMetricsSnapshot},
    delegation::{DelegationMetrics, DelegationMetricsSnapshot},
    endpoint::{EndpointHealthSnapshot, EndpointMetrics},
    http::{HttpMetrics, HttpMetricsSnapshot},
    icc::{IccMetrics, IccMetricsSnapshot},
    system::{SystemMetrics, SystemMetricsSnapshot},
    timer::{TimerMetrics, TimerMetricsSnapshot},
};

///
/// MetricsOps
///
/// Aggregation facade over the ops-internal metrics subsystems.
///
/// NOTE:
/// Individual metrics (AccessMetrics, HttpMetrics, IccMetrics, etc.) are modeled
/// as concrete, state-owning subsystems rather than `*Ops` facades, because they
/// directly own their in-memory storage and semantics.
///
/// `MetricsOps` exists solely as a convenience aggregator to provide a stable,
/// import-friendly snapshot surface for callers that need multiple metrics at
/// once, without exposing internal storage details or requiring many imports.
///

pub struct MetricsOps;

impl MetricsOps {
    #[must_use]
    pub fn access_snapshot() -> AccessMetricsSnapshot {
        AccessMetrics::snapshot()
    }

    #[must_use]
    pub fn delegation_snapshot() -> DelegationMetricsSnapshot {
        DelegationMetrics::snapshot()
    }

    #[must_use]
    pub fn endpoint_health_snapshot() -> EndpointHealthSnapshot {
        EndpointMetrics::health_snapshot()
    }

    #[must_use]
    pub fn http_snapshot() -> HttpMetricsSnapshot {
        HttpMetrics::snapshot()
    }

    #[must_use]
    pub fn icc_snapshot() -> IccMetricsSnapshot {
        IccMetrics::snapshot()
    }

    #[must_use]
    pub fn system_snapshot() -> SystemMetricsSnapshot {
        SystemMetrics::snapshot()
    }

    #[must_use]
    pub fn timer_snapshot() -> TimerMetricsSnapshot {
        TimerMetrics::snapshot()
    }
}
