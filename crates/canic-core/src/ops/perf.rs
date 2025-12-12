pub use crate::perf::PerfEntry;

use crate::{cdk::candid::CandidType, perf, types::PageRequest};
use serde::{Deserialize, Serialize};

///
/// PerfSnapshot
/// Paginated view of perf counters keyed by label.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct PerfSnapshot {
    pub entries: Vec<PerfEntry>,
    pub total: u64,
}

///
/// PerfOps
///

pub struct PerfOps;

impl PerfOps {
    pub(crate) fn record(label: &str, delta: u64) {
        perf::record(label, delta);
    }

    #[must_use]
    pub fn snapshot(request: PageRequest) -> PerfSnapshot {
        let request = request.clamped();
        let offset = usize::try_from(request.offset).unwrap_or(usize::MAX);
        let limit = usize::try_from(request.limit).unwrap_or(usize::MAX);

        let entries = perf::entries();
        let total = entries.len() as u64;
        let entries = entries.into_iter().skip(offset).take(limit).collect();

        PerfSnapshot { entries, total }
    }
}
