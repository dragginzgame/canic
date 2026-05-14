use crate::metrics::model::{MetricEntry, MetricValue};
use canic_host::response_parse::{
    RECORD_MARKER, candid_record_blocks, find_field, parse_json_u64, parse_json_u128,
    parse_u64_digits, parse_u128_digits, quoted_strings, response_candid, text_after,
};

pub fn parse_metrics_page(output: &str) -> Option<Vec<MetricEntry>> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    if let Some(entries) = parse_metrics_page_json(&value) {
        return Some(entries);
    }
    response_candid(&value).and_then(parse_metrics_page_text)
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

fn parse_metrics_page_text(output: &str) -> Option<Vec<MetricEntry>> {
    let mut entries = Vec::new();
    for chunk in candid_record_blocks(output) {
        if !(chunk[RECORD_MARKER.len()..]
            .trim_start()
            .starts_with("\"principal\" =")
            && chunk.contains("labels = vec")
            && chunk.contains("value = variant"))
        {
            continue;
        }
        entries.push(MetricEntry {
            labels: parse_candid_labels(chunk)?,
            principal: parse_candid_principal(chunk),
            value: parse_candid_metric_value(chunk)?,
        });
    }
    Some(entries)
}

fn parse_candid_labels(chunk: &str) -> Option<Vec<String>> {
    let (_, after_field) = chunk.split_once("labels = vec")?;
    let (_, after_open) = after_field.split_once('{')?;
    let (labels, _) = after_open.split_once("};")?;
    Some(quoted_strings(labels))
}

fn parse_candid_principal(chunk: &str) -> Option<String> {
    let (_, after_field) = chunk.split_once("\"principal\" =")?;
    let value = after_field.trim_start();
    if value.starts_with("null") {
        return None;
    }
    quoted_strings(value).into_iter().next()
}

fn parse_candid_metric_value(chunk: &str) -> Option<MetricValue> {
    if let Some(value) = text_after(chunk, "Count =").and_then(parse_u64_digits) {
        return Some(MetricValue::Count { count: value });
    }
    if let Some(value) = text_after(chunk, "U128 =").and_then(parse_u128_digits) {
        return Some(MetricValue::U128 { value });
    }
    if chunk.contains("CountAndU64") {
        let count = text_after(chunk, "count =").and_then(parse_u64_digits)?;
        let value_u64 = text_after(chunk, "value_u64 =").and_then(parse_u64_digits)?;
        return Some(MetricValue::CountAndU64 { count, value_u64 });
    }
    None
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
