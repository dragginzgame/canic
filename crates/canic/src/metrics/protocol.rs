//! Passive contract for the shared metrics query.
use super::{Counter, Snapshot};
use serde::{Deserialize, Serialize};

/// One bounded source selection, authorized by the same canister guard.
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
#[derive(Debug, Deserialize, Serialize)]
pub enum Request {
    /// Aggregate timer counters for this incarnation.
    Timers,
    /// One page of completed synchronous update measurements.
    Endpoints(Option<String>),
    /// Committed Blob command outcomes.
    Blob,
    /// Database query and commit totals for this incarnation.
    Db,
    /// Scheduled job outcomes for this incarnation.
    Jobs,
    /// Supplier-managed fresh entropy requests for this incarnation.
    Entropy,
}

/// Command outcomes observed by the local Blob recorder.
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
#[cfg_attr(feature = "inspection", wire(fixed))]
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
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
#[cfg_attr(feature = "inspection", wire(fixed))]
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
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
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
    /// Database query and commit totals for this incarnation.
    Db {
        /// Returned query terminals, including refusals.
        queries: Counter,
        /// Query terminals returning an error.
        query_failures: Counter,
        /// Candidates examined by successful queries.
        examined: Counter,
        /// Rows fetched by successful queries.
        fetched: Counter,
        /// Results delivered by successful queries.
        returned: Counter,
        /// Encoded result bytes delivered.
        output_bytes: Counter,
        /// Returned commit calls, including refusals.
        commits: Counter,
        /// Commit calls returning an error.
        commit_failures: Counter,
        /// Aggregate job start calls.
        job_starts: Counter,
        /// Aggregate job resume calls.
        job_resumes: Counter,
        /// Aggregate job calls returning an error.
        job_failures: Counter,
    },
    /// Scheduled job outcomes for this incarnation.
    Jobs {
        /// Attempts admitted to a business callback.
        started: Counter,
        /// Started attempts numbered two or higher.
        retry_attempts: Counter,
        /// Dispatches suppressed by lateness policy.
        skipped_late: Counter,
        /// Normal returns with no business work.
        no_work: Counter,
        /// Normal returns reporting success.
        succeeded: Counter,
        /// Normal returns reporting a settled failure.
        expected_failures: Counter,
        /// Normal returns permitting a retry.
        retryable_failures: Counter,
    },
    /// Supplier-managed fresh entropy requests for this incarnation.
    Entropy {
        /// Fresh requests polled in replicated execution.
        attempts: Counter,
        /// Fresh requests returning a seed.
        succeeded: Counter,
        /// Fresh requests returning an error.
        failed: Counter,
        /// Polled fresh requests dropped before return.
        cancelled: Counter,
    },
}

/// Shared query for the canister's local metrics.
pub struct ReadMetrics;

impl crate::method::Contract for ReadMetrics {
    const NAME: &'static str = "canic_metrics";
    const ID: u64 = ic_wire::method_id("metrics");
    const REQUEST: ic_wire::Limits = ic_wire::Limits {
        bytes: 512,
        allocation: 16_384,
        nodes: 256,
        depth: 8,
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
/// ObserveChildMetrics
/// Root controller reads one source from a ready managed child slot.
///

pub struct ObserveChildMetrics;

impl crate::method::Contract for ObserveChildMetrics {
    const NAME: &'static str = "observe_child_metrics";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    const REQUEST: ic_wire::Limits = ReadMetrics::REQUEST;
    const REPLY: ic_wire::Limits = ReadMetrics::REPLY;
    type Input = (u32, Request);
    type Output = Report;
}
