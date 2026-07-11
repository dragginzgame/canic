use crate::cycles::model::{
    CycleTopupEventPage, CycleTopupEventSample, CycleTopupStatus, CycleTrackerPage,
    CycleTrackerSample,
};
use canic_host::response_parse::{find_field, parse_json_u64, parse_json_u128};
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CycleResponseKind {
    Tracker,
    Topup,
}

impl CycleResponseKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Tracker => "cycle tracker",
            Self::Topup => "cycle top-up",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum CyclesParseError {
    InvalidJson {
        kind: CycleResponseKind,
        error: String,
    },
    MissingEntries(CycleResponseKind),
    InvalidEntries(CycleResponseKind),
    InvalidTotal(CycleResponseKind),
    InvalidEntryField {
        kind: CycleResponseKind,
        index: usize,
        field: &'static str,
    },
}

impl fmt::Display for CyclesParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson { kind, error } => {
                write!(
                    formatter,
                    "{} response has invalid JSON: {error}",
                    kind.label()
                )
            }
            Self::MissingEntries(kind) => {
                write!(formatter, "{} response is missing `entries`", kind.label())
            }
            Self::InvalidEntries(kind) => {
                write!(
                    formatter,
                    "{} response `entries` must be an array",
                    kind.label()
                )
            }
            Self::InvalidTotal(kind) => {
                write!(formatter, "{} response has invalid `total`", kind.label())
            }
            Self::InvalidEntryField { kind, index, field } => write!(
                formatter,
                "{} entry {index} has invalid `{field}`",
                kind.label()
            ),
        }
    }
}

impl Error for CyclesParseError {}

pub(super) fn parse_cycle_tracker_page(output: &str) -> Result<CycleTrackerPage, CyclesParseError> {
    let value = parse_json_response(output, CycleResponseKind::Tracker)?;
    parse_cycle_tracker_page_json(&value)
}

fn parse_cycle_tracker_page_json(
    value: &serde_json::Value,
) -> Result<CycleTrackerPage, CyclesParseError> {
    let kind = CycleResponseKind::Tracker;
    let entries_value =
        find_field(value, "entries").ok_or(CyclesParseError::MissingEntries(kind))?;
    let entries = entries_value
        .as_array()
        .ok_or(CyclesParseError::InvalidEntries(kind))?
        .iter()
        .enumerate()
        .map(|(index, value)| parse_cycle_tracker_sample_json(value, index))
        .collect::<Result<Vec<_>, _>>()?;
    let total = parse_total(value, kind, entries.len())?;

    Ok(CycleTrackerPage { entries, total })
}

fn parse_cycle_tracker_sample_json(
    value: &serde_json::Value,
    index: usize,
) -> Result<CycleTrackerSample, CyclesParseError> {
    let invalid_field = |field| CyclesParseError::InvalidEntryField {
        kind: CycleResponseKind::Tracker,
        index,
        field,
    };
    Ok(CycleTrackerSample {
        timestamp_secs: find_field(value, "timestamp_secs")
            .and_then(parse_json_u64)
            .ok_or_else(|| invalid_field("timestamp_secs"))?,
        cycles: find_field(value, "cycles")
            .and_then(parse_json_u128)
            .ok_or_else(|| invalid_field("cycles"))?,
    })
}

pub(super) fn parse_topup_event_page(
    output: &str,
) -> Result<CycleTopupEventPage, CyclesParseError> {
    let value = parse_json_response(output, CycleResponseKind::Topup)?;
    parse_topup_event_page_json(&value)
}

fn parse_topup_event_page_json(
    value: &serde_json::Value,
) -> Result<CycleTopupEventPage, CyclesParseError> {
    let kind = CycleResponseKind::Topup;
    let entries = find_field(value, "entries")
        .ok_or(CyclesParseError::MissingEntries(kind))?
        .as_array()
        .ok_or(CyclesParseError::InvalidEntries(kind))?
        .iter()
        .enumerate()
        .map(|(index, value)| parse_topup_event_json(value, index))
        .collect::<Result<Vec<_>, _>>()?;
    let total = parse_total(value, kind, entries.len())?;

    Ok(CycleTopupEventPage { entries, total })
}

fn parse_topup_event_json(
    value: &serde_json::Value,
    index: usize,
) -> Result<CycleTopupEventSample, CyclesParseError> {
    let invalid_field = |field| CyclesParseError::InvalidEntryField {
        kind: CycleResponseKind::Topup,
        index,
        field,
    };
    Ok(CycleTopupEventSample {
        timestamp_secs: find_field(value, "timestamp_secs")
            .and_then(parse_json_u64)
            .ok_or_else(|| invalid_field("timestamp_secs"))?,
        transferred_cycles: parse_optional_u128_field(find_field(value, "transferred_cycles"))
            .map_err(|()| invalid_field("transferred_cycles"))?,
        status: find_field(value, "status")
            .and_then(parse_topup_status_json)
            .ok_or_else(|| invalid_field("status"))?,
    })
}

fn parse_json_response(
    output: &str,
    kind: CycleResponseKind,
) -> Result<serde_json::Value, CyclesParseError> {
    serde_json::from_str(output).map_err(|error| CyclesParseError::InvalidJson {
        kind,
        error: error.to_string(),
    })
}

fn parse_total(
    value: &serde_json::Value,
    kind: CycleResponseKind,
    entries_len: usize,
) -> Result<u64, CyclesParseError> {
    let Some(total) = find_field(value, "total") else {
        return Ok(entries_len as u64);
    };
    parse_json_u64(total).ok_or(CyclesParseError::InvalidTotal(kind))
}

fn parse_optional_u128_field(value: Option<&serde_json::Value>) -> Result<Option<u128>, ()> {
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Array(values)) if values.is_empty() => Ok(None),
        Some(value) => parse_optional_u128(value).map(Some).ok_or(()),
    }
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
