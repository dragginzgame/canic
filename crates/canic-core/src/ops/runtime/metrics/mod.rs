pub mod access;
pub mod endpoint;
pub mod http;
pub mod icc;
pub mod store;
pub mod system;
pub mod timer;

use {
    access::AccessMetricsSnapshot, endpoint::EndpointHealthSnapshot, http::HttpMetricsSnapshot,
    icc::IccMetricsSnapshot, system::SystemMetricsSnapshot, timer::TimerMetricsSnapshot,
};

///
/// MetricsOps
///

pub struct MetricsOps;

impl MetricsOps {
    #[must_use]
    pub fn access_snapshot() -> AccessMetricsSnapshot {
        access::snapshot()
    }

    #[must_use]
    pub fn endpoint_health_snapshot() -> EndpointHealthSnapshot {
        endpoint::health_snapshot()
    }

    #[must_use]
    pub fn http_snapshot() -> HttpMetricsSnapshot {
        http::snapshot()
    }

    #[must_use]
    pub fn icc_snapshot() -> IccMetricsSnapshot {
        icc::snapshot()
    }

    #[must_use]
    pub fn system_snapshot() -> SystemMetricsSnapshot {
        system::snapshot()
    }

    #[must_use]
    pub fn timer_snapshot() -> TimerMetricsSnapshot {
        timer::snapshot()
    }
}
