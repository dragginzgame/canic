use crate::replica_query::parse_ready_json_value;
use crate::response_parse::{
    field_value_after_equals, parse_candid_text_like_field, response_candid,
};
use canic_core::dto::state::BootstrapStatusResponse;
use serde_json::Value;

pub(in crate::install_root) type BootstrapStatusSnapshot = BootstrapStatusResponse;

// Accept both plain-bool and wrapped-result JSON shapes from `icp --output json`.
pub(in crate::install_root) fn parse_root_ready_value(data: &Value) -> bool {
    parse_ready_json_value(data)
}

pub(in crate::install_root) fn parse_bootstrap_status_value(
    data: &Value,
) -> Option<BootstrapStatusSnapshot> {
    serde_json::from_value::<BootstrapStatusResponse>(data.clone())
        .ok()
        .or_else(|| {
            data.get("Ok")
                .cloned()
                .and_then(|ok| serde_json::from_value::<BootstrapStatusResponse>(ok).ok())
        })
        .or_else(|| response_candid(data).and_then(parse_bootstrap_status_candid))
}

fn parse_bootstrap_status_candid(candid: &str) -> Option<BootstrapStatusSnapshot> {
    let ready = parse_bootstrap_ready_field(candid)?;
    let phase = parse_candid_text_like_field(candid, "3_253_282_875")
        .or_else(|| parse_candid_text_like_field(candid, "phase"))
        .unwrap_or_else(|| {
            if ready {
                "ready".to_string()
            } else {
                "unknown".to_string()
            }
        });
    let last_error = parse_candid_text_like_field(candid, "89_620_959")
        .or_else(|| parse_candid_text_like_field(candid, "last_error"));

    Some(BootstrapStatusResponse {
        ready,
        phase,
        last_error,
    })
}

fn parse_bootstrap_ready_field(candid: &str) -> Option<bool> {
    let value = field_value_after_equals(candid, "3_870_990_435")
        .or_else(|| field_value_after_equals(candid, "ready"))?;
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        None
    }
}
