use crate::{
    dto::page::{Page, PageRequest},
    ops::perf::PerfOps,
    perf::PerfEntry,
};

#[must_use]
pub fn snapshot(request: PageRequest) -> Page<PerfEntry> {
    PerfOps::snapshot(request)
}
