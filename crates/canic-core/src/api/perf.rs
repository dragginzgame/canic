use crate::{
    dto::page::{Page, PageRequest},
    perf::PerfEntry,
    workflow,
};

#[must_use]
pub fn snapshot(request: PageRequest) -> Page<PerfEntry> {
    workflow::query::metrics::metrics_perf_page(request)
}
