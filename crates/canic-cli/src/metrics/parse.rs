//! Module: metrics::parse
//!
//! Responsibility: decode typed metric pages and project them into CLI report rows.
//! Does not own: ICP command execution, metric collection, or report rendering.
//! Boundary: accepts only the canonical ICP JSON envelope with typed Candid bytes.

use crate::metrics::model::{MetricEntry, MetricValue};
#[cfg(test)]
use candid::{CandidType, Deserialize};
use canic_core::dto::{
    metrics::{MetricEntry as MetricEntryDto, MetricValue as MetricValueDto},
    page::Page,
};
#[cfg(test)]
use canic_host::icp::{IcpJsonResponseError, decode_json_result_response};

#[cfg(test)]
#[derive(CandidType, Deserialize)]
pub(super) enum MetricsStatusResponse {
    Metrics(Page<MetricEntryDto>),
}

#[cfg(test)]
pub(super) fn parse_metrics_page(output: &str) -> Result<Vec<MetricEntry>, IcpJsonResponseError> {
    let response = decode_json_result_response::<MetricsStatusResponse>(output)?;
    let MetricsStatusResponse::Metrics(page) = response;
    Ok(metric_page(page))
}

pub(super) fn metric_page(page: Page<MetricEntryDto>) -> Vec<MetricEntry> {
    page.entries.into_iter().map(metric_entry).collect()
}

fn metric_entry(entry: MetricEntryDto) -> MetricEntry {
    MetricEntry {
        labels: entry.labels,
        principal: entry.principal.map(|principal| principal.to_text()),
        value: metric_value(entry.value),
    }
}

const fn metric_value(value: MetricValueDto) -> MetricValue {
    match value {
        MetricValueDto::Count(count) => MetricValue::Count { count },
        MetricValueDto::CountAndU64 { count, value_u64 } => {
            MetricValue::CountAndU64 { count, value_u64 }
        }
        MetricValueDto::U128(value) => MetricValue::U128 { value },
    }
}
