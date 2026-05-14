use crate::cycles::model::{
    CycleTopupEventPage, CycleTopupEventSample, CycleTopupStatus, CycleTrackerPage,
    CycleTrackerSample,
};
use canic_host::response_parse::{
    field_value_after_equals, find_field, parse_json_u64, parse_json_u128, parse_u64_digits,
    parse_u128_digits, response_candid,
};

pub(super) fn parse_cycle_tracker_page(output: &str) -> Option<CycleTrackerPage> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    if let Some(page) = parse_cycle_tracker_page_json(&value) {
        return Some(page);
    }
    response_candid(&value).and_then(parse_cycle_tracker_page_text)
}

fn parse_cycle_tracker_page_json(value: &serde_json::Value) -> Option<CycleTrackerPage> {
    let entries_value = find_field(value, "entries")?;
    let entries = entries_value
        .as_array()?
        .iter()
        .map(parse_cycle_tracker_sample_json)
        .collect::<Option<Vec<_>>>()?;
    let total = find_field(value, "total")
        .and_then(parse_json_u64)
        .unwrap_or(entries.len() as u64);

    Some(CycleTrackerPage { entries, total })
}

fn parse_cycle_tracker_sample_json(value: &serde_json::Value) -> Option<CycleTrackerSample> {
    Some(CycleTrackerSample {
        timestamp_secs: find_field(value, "timestamp_secs").and_then(parse_json_u64)?,
        cycles: find_field(value, "cycles").and_then(parse_json_u128)?,
    })
}

pub(super) fn parse_topup_event_page(output: &str) -> Option<CycleTopupEventPage> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    if let Some(page) = parse_topup_event_page_json(&value) {
        return Some(page);
    }
    response_candid(&value).and_then(parse_topup_event_page_text)
}

fn parse_topup_event_page_json(value: &serde_json::Value) -> Option<CycleTopupEventPage> {
    let entries = find_field(value, "entries")?
        .as_array()?
        .iter()
        .map(parse_topup_event_json)
        .collect::<Option<Vec<_>>>()?;
    let total = find_field(value, "total")
        .and_then(parse_json_u64)
        .unwrap_or(entries.len() as u64);

    Some(CycleTopupEventPage { entries, total })
}

fn parse_topup_event_json(value: &serde_json::Value) -> Option<CycleTopupEventSample> {
    Some(CycleTopupEventSample {
        timestamp_secs: find_field(value, "timestamp_secs").and_then(parse_json_u64)?,
        transferred_cycles: find_field(value, "transferred_cycles").and_then(parse_optional_u128),
        status: find_field(value, "status").and_then(parse_topup_status_json)?,
    })
}

fn parse_optional_u128(value: &serde_json::Value) -> Option<u128> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::Object(map) => map.values().find_map(parse_json_u128),
        serde_json::Value::Array(values) => values.iter().find_map(parse_json_u128),
        _ => parse_json_u128(value),
    }
}

fn parse_topup_status_json(value: &serde_json::Value) -> Option<CycleTopupStatus> {
    match value {
        serde_json::Value::String(status) => parse_topup_status(status),
        serde_json::Value::Object(map) => map.keys().find_map(|key| parse_topup_status(key)),
        serde_json::Value::Array(values) => values.iter().find_map(parse_topup_status_json),
        _ => None,
    }
}

pub(super) fn parse_cycle_tracker_page_text(output: &str) -> Option<CycleTrackerPage> {
    let mut entries = Vec::new();
    for chunk in output.split("record") {
        if !(chunk.contains("timestamp_secs") && chunk.contains("cycles")) {
            continue;
        }
        let timestamp_secs =
            field_number_after(chunk, "timestamp_secs").and_then(parse_u64_digits)?;
        let cycles = field_number_after(chunk, "cycles").and_then(parse_u128_digits)?;
        entries.push(CycleTrackerSample {
            timestamp_secs,
            cycles,
        });
    }
    let total = field_number_after(output, "total")
        .and_then(parse_u64_digits)
        .unwrap_or(entries.len() as u64);
    Some(CycleTrackerPage { entries, total })
}

pub(super) fn parse_topup_event_page_text(output: &str) -> Option<CycleTopupEventPage> {
    let mut entries = Vec::new();
    for chunk in output.split("record") {
        if !(chunk.contains("timestamp_secs") && chunk.contains("status")) {
            continue;
        }
        let timestamp_secs =
            field_number_after(chunk, "timestamp_secs").and_then(parse_u64_digits)?;
        let transferred_cycles =
            field_number_after(chunk, "transferred_cycles").and_then(parse_u128_digits);
        let status = parse_topup_status(chunk)?;
        entries.push(CycleTopupEventSample {
            timestamp_secs,
            transferred_cycles,
            status,
        });
    }
    let total = field_number_after(output, "total")
        .and_then(parse_u64_digits)
        .unwrap_or(entries.len() as u64);
    Some(CycleTopupEventPage { entries, total })
}

fn parse_topup_status(text: &str) -> Option<CycleTopupStatus> {
    if text.contains("RequestOk") || text.contains("request_ok") {
        Some(CycleTopupStatus::RequestOk)
    } else if text.contains("RequestErr") || text.contains("request_err") {
        Some(CycleTopupStatus::RequestErr)
    } else if text.contains("RequestScheduled") || text.contains("request_scheduled") {
        Some(CycleTopupStatus::RequestScheduled)
    } else {
        None
    }
}

fn field_number_after<'a>(text: &'a str, field: &str) -> Option<&'a str> {
    field_value_after_equals(text, field)
}
