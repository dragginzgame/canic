//! Lossless metric projection conversions at the transport boundary.
use crate::dto::{metric_page as wire, metrics as source, page::Page};
impl From<source::MetricEntry> for wire::MetricEntry {
    fn from(value: source::MetricEntry) -> Self {
        Self {
            labels: value.labels,
            principal: value.principal.map(Into::into),
            value: value.value,
        }
    }
}
impl From<wire::MetricEntry> for source::MetricEntry {
    fn from(value: wire::MetricEntry) -> Self {
        Self {
            labels: value.labels,
            principal: value.principal.map(Into::into),
            value: value.value,
        }
    }
}
impl From<Page<source::MetricEntry>> for Page<wire::MetricEntry> {
    fn from(value: Page<source::MetricEntry>) -> Self {
        Self {
            entries: value.entries.into_iter().map(Into::into).collect(),
            total: value.total,
        }
    }
}
impl From<Page<wire::MetricEntry>> for Page<source::MetricEntry> {
    fn from(value: Page<wire::MetricEntry>) -> Self {
        Self {
            entries: value.entries.into_iter().map(Into::into).collect(),
            total: value.total,
        }
    }
}
