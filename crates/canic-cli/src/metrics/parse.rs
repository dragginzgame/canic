use crate::metrics::model::{MetricEntry, MetricValue};
use canic_host::response_parse::{find_field, parse_json_u64, parse_json_u128};
use std::{error::Error, fmt};

#[derive(Debug, Eq, PartialEq)]
pub(super) enum MetricsParseError {
    InvalidJson(String),
    MissingEntries,
    InvalidEntries,
    InvalidEntryField { index: usize, field: &'static str },
}

impl fmt::Display for MetricsParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(error) => write!(formatter, "invalid JSON: {error}"),
            Self::MissingEntries => formatter.write_str("missing `entries` field"),
            Self::InvalidEntries => formatter.write_str("`entries` must be an array"),
            Self::InvalidEntryField { index, field } => {
                write!(formatter, "entry {index} has invalid `{field}`")
            }
        }
    }
}

impl Error for MetricsParseError {}

pub(super) fn parse_metrics_page(output: &str) -> Result<Vec<MetricEntry>, MetricsParseError> {
    let value = serde_json::from_str::<serde_json::Value>(output)
        .map_err(|error| MetricsParseError::InvalidJson(error.to_string()))?;
    parse_metrics_page_json(&value)
}

fn parse_metrics_page_json(
    value: &serde_json::Value,
) -> Result<Vec<MetricEntry>, MetricsParseError> {
    find_field(value, "entries")
        .ok_or(MetricsParseError::MissingEntries)?
        .as_array()
        .ok_or(MetricsParseError::InvalidEntries)?
        .iter()
        .enumerate()
        .map(|(index, value)| parse_metric_entry_json(value, index))
        .collect()
}

fn parse_metric_entry_json(
    value: &serde_json::Value,
    index: usize,
) -> Result<MetricEntry, MetricsParseError> {
    let invalid_field = |field| MetricsParseError::InvalidEntryField { index, field };
    let labels = find_field(value, "labels")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| invalid_field("labels"))?
        .iter()
        .map(|value| value.as_str().map(str::to_string))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| invalid_field("labels"))?;
    let principal = match find_field(value, "principal") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Array(values)) if values.is_empty() => None,
        Some(value) => Some(parse_principal_json(value).ok_or_else(|| invalid_field("principal"))?),
    };
    let metric_value = find_field(value, "value")
        .and_then(parse_metric_value_json)
        .ok_or_else(|| invalid_field("value"))?;

    Ok(MetricEntry {
        labels,
        principal,
        value: metric_value,
    })
}

fn parse_principal_json(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Object(map) => map.values().find_map(parse_principal_json),
        serde_json::Value::Array(values) => values.iter().find_map(parse_principal_json),
        _ => None,
    }
}

fn parse_metric_value_json(value: &serde_json::Value) -> Option<MetricValue> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(value) = map.get("Count").and_then(parse_json_u64) {
                return Some(MetricValue::Count { count: value });
            }
            if let Some(value) = map.get("U128").and_then(parse_json_u128) {
                return Some(MetricValue::U128 { value });
            }
            if let Some(value) = map.get("CountAndU64") {
                let count = find_field(value, "count").and_then(parse_json_u64)?;
                let value_u64 = find_field(value, "value_u64").and_then(parse_json_u64)?;
                return Some(MetricValue::CountAndU64 { count, value_u64 });
            }
            if let (Some(count), Some(value_u64)) = (
                map.get("count").and_then(parse_json_u64),
                map.get("value_u64").and_then(parse_json_u64),
            ) {
                return Some(MetricValue::CountAndU64 { count, value_u64 });
            }
            map.values().find_map(parse_metric_value_json)
        }
        serde_json::Value::Array(values) => values.iter().find_map(parse_metric_value_json),
        _ => None,
    }
}
