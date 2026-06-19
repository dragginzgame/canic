//! Module: ops::perf
//!
//! Responsibility: record approved runtime performance samples.
//! Does not own: metrics projection, timer scheduling, or perf table storage.
//! Boundary: ops facade over the shared perf recording surface.

use crate::perf;

///
/// PerfOps
///
/// Operations-layer facade for performance sample recording.
///

pub struct PerfOps;

impl PerfOps {
    // Record a timer perf sample into the shared perf table.
    pub(crate) fn record(label: &str, delta: u64) {
        perf::record_timer(label, delta);
    }
}
