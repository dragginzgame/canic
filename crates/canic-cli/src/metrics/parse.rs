//! Module: metrics::parse
//!
//! Responsibility: decode typed metric pages and project them into CLI report rows.
//! Does not own: ICP command execution, metric collection, or report rendering.
//! Boundary: accepts only the canonical ICP JSON envelope with typed Candid bytes.

use crate::metrics::model::{MetricEntry, MetricValue};
use canic_core::dto::{
    metrics::{MetricEntry as MetricEntryDto, MetricValue as MetricValueDto},
    page::Page,
};
use canic_host::icp::{IcpJsonResponseError, decode_json_result_response};

pub(super) fn parse_metrics_page(output: &str) -> Result<Vec<MetricEntry>, IcpJsonResponseError> {
    let page = decode_json_result_response::<Page<MetricEntryDto>>(output)?;
    Ok(page.entries.into_iter().map(metric_entry).collect())
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
