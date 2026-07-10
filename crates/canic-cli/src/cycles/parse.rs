use crate::cycles::model::{
    CycleTopupEventPage, CycleTopupEventSample, CycleTopupStatus, CycleTrackerPage,
    CycleTrackerSample,
};
use canic_host::response_parse::{find_field, parse_json_u64, parse_json_u128};

pub(super) fn parse_cycle_tracker_page(output: &str) -> Option<CycleTrackerPage> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    parse_cycle_tracker_page_json(&value)
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
    parse_topup_event_page_json(&value)
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

fn parse_topup_status(text: &str) -> Option<CycleTopupStatus> {
    match text {
        "RequestOk" | "request_ok" => Some(CycleTopupStatus::RequestOk),
        "RequestErr" | "request_err" => Some(CycleTopupStatus::RequestErr),
        "RequestScheduled" | "request_scheduled" => Some(CycleTopupStatus::RequestScheduled),
        _ => None,
    }
}
