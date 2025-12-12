pub use crate::interface::ic::timer::{PerfEntry, PerfSnapshot};

use crate::interface::ic::timer;

pub struct PerfOps;

impl PerfOps {
    pub(crate) fn record(label: &str, delta: u64) {
        timer::record(label, delta);
    }

    #[must_use]
    pub fn snapshot() -> PerfSnapshot {
        timer::snapshot()
    }
}
