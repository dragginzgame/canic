//! Passive contract for the shared metrics query.
use super::{Counter, Snapshot};
use serde::{Deserialize, Serialize};

/// One bounded source selection, authorized by the same canister guard.
#[derive(Debug, Deserialize, Serialize)]
pub enum Request {
    /// Aggregate timer counters for this incarnation.
    Timers,
    /// One page of completed synchronous update measurements.
    Endpoints(Option<String>),
    /// Committed Blob command outcomes.
    Blob,
}

/// Command outcomes observed by the local Blob recorder.
#[derive(Debug, Deserialize, Serialize)]
pub struct BlobCommand {
    /// Blob command kind.
    pub command: String,
    /// Successful calls, including exact retries.
    pub succeeded: Counter,
    /// Calls refused for lack of authority.
    pub denied: Counter,
    /// Capacity refusals.
    pub capacity: Counter,
    /// Expired grant refusals.
    pub expired: Counter,
    /// Other ordinary refusals.
    pub other_refusals: Counter,
}

/// Declared object sizes recorded on first successful registration.
#[derive(Debug, Deserialize, Serialize)]
pub struct BlobSizes {
    /// Inclusive byte limits of the four supplier buckets.
    pub upper_bounds: [u64; 4],
    /// Observations in each bounded bucket.
    pub buckets: [Counter; 4],
    /// Observations above the last bound.
    pub above: Counter,
}

/// A single source report. Unavailable means the source supplied no report.
#[derive(Debug, Deserialize, Serialize)]
pub enum Report {
    /// No reader or recorder for this source.
    Unavailable,
    /// Timer observations.
    Timers(Snapshot),
    /// Endpoint performance observations.
    Endpoints(super::performance::Page),
    /// One row for each supported Blob command.
    Blob {
        /// Outcomes in supplier command order.
        commands: Vec<BlobCommand>,
        /// First-registration size distribution.
        sizes: BlobSizes,
    },
}

/// Shared query for the canister's local metrics.
pub struct ReadMetrics;

impl crate::method::Contract for ReadMetrics {
    const NAME: &'static str = "metrics";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REQUEST: ic_wire::Limits = ic_wire::Limits {
        nodes: 256,
        ..crate::method::LIMITS
    };
    const REPLY: ic_wire::Limits = ic_wire::Limits {
        bytes: 4096,
        allocation: 131_072,
        nodes: 4096,
        depth: 8,
    };
    type Input = Request;
    type Output = Report;
}

///
/// ChildMetrics
/// Root controller reads one source from a ready managed child slot.
///

pub struct ChildMetrics;

impl crate::method::Contract for ChildMetrics {
    const NAME: &'static str = "child_metrics";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REQUEST: ic_wire::Limits = ReadMetrics::REQUEST;
    const REPLY: ic_wire::Limits = ReadMetrics::REPLY;
    type Input = (u8, Request);
    type Output = Report;
}
