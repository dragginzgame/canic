use crate::perf;

///
/// PerfOps
///

pub struct PerfOps;

impl PerfOps {
    // Record a timer perf sample into the shared perf table.
    pub(crate) fn record(label: &str, delta: u64) {
        perf::record_timer(label, delta);
    }
}
