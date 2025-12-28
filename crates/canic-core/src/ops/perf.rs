use crate::{
    dto::page::{Page, PageRequest},
    perf,
};

pub use crate::perf::PerfEntry;

///
/// PerfOps
///

pub struct PerfOps;

impl PerfOps {
    pub(crate) fn record(label: &str, delta: u64) {
        perf::record_timer(label, delta);
    }

    #[must_use]
    pub fn snapshot(request: PageRequest) -> Page<PerfEntry> {
        let request = request.clamped();
        let offset = usize::try_from(request.offset).unwrap_or(usize::MAX);
        let limit = usize::try_from(request.limit).unwrap_or(usize::MAX);

        let entries = perf::entries();
        let total = entries.len() as u64;
        let entries = entries.into_iter().skip(offset).take(limit).collect();

        Page { entries, total }
    }
}
