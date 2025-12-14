pub mod access;
pub mod http;
pub mod icc;
pub mod system;
pub mod timer;

pub use access::*;
pub use http::*;
pub use icc::*;
pub use system::*;
pub use timer::*;

use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// MetricsReport
/// Composite metrics view bundling action, ICC, HTTP, and timer counters.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct MetricsReport {
    pub system: SystemMetricsSnapshot,
    pub icc: IccMetricsSnapshot,
    pub http: HttpMetricsSnapshot,
    pub timer: TimerMetricsSnapshot,
    pub access: AccessMetricsSnapshot,
}
