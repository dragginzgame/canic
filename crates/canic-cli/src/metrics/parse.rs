use crate::metrics::model::{MetricEntry, MetricValue};
use canic_host::response_parse::{find_field, parse_json_u64, parse_json_u128};

pub(super) fn parse_metrics_page(output: &str) -> Option<Vec<MetricEntry>> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    parse_metrics_page_json(&value)
}

fn parse_metrics_page_json(value: &serde_json::Value) -> Option<Vec<MetricEntry>> {
    find_field(value, "entries")?
        .as_array()?
        .iter()
        .map(parse_metric_entry_json)
        .collect()
}

fn parse_metric_entry_json(value: &serde_json::Value) -> Option<MetricEntry> {
    Some(MetricEntry {
        labels: find_field(value, "labels")?
            .as_array()?
            .iter()
            .map(|value| value.as_str().map(str::to_string))
            .collect::<Option<Vec<_>>>()?,
        principal: find_field(value, "principal").and_then(parse_principal_json),
        value: find_field(value, "value").and_then(parse_metric_value_json)?,
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
