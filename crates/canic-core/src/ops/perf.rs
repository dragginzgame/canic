use crate::perf::{self, PerfEntry};

///
/// PerfOps
///

pub struct PerfOps;

///
/// PerfSnapshot
///

#[derive(Clone, Debug)]
pub struct PerfSnapshot {
    pub entries: Vec<PerfEntry>,
    pub total: u64,
}

impl PerfOps {
    pub(crate) fn record(label: &str, delta: u64) {
        perf::record_timer(label, delta);
    }

    #[must_use]
    pub fn snapshot() -> PerfSnapshot {
        let entries = perf::entries();
        let total = entries.len() as u64;

        PerfSnapshot { entries, total }
    }
}
