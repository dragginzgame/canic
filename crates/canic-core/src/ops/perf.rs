use crate::perf::{self, PerfEntry};

///
/// PerfSnapshot
///

#[derive(Clone, Debug)]
pub struct PerfSnapshot {
    pub entries: Vec<PerfEntry>,
}

///
/// PerfOps
///

pub struct PerfOps;

impl PerfOps {
    pub(crate) fn record(label: &str, delta: u64) {
        perf::record_timer(label, delta);
    }

    #[must_use]
    pub fn snapshot() -> PerfSnapshot {
        let entries = perf::entries();

        PerfSnapshot { entries }
    }
}
