//! Module: canic_cli::auth::codec
//!
//! Responsibility: decode delegated-auth responses and build small Candid arguments.
//! Does not own: command execution, transport, or operator rendering.

use super::{
    AuthCommandError, AuthIssuerObservedStatus, AuthRenewalActiveAttemptStatus,
    AuthRenewalStateStatus, AuthRenewalStatusSummary, AuthRenewalTemplateStatus,
};
use candid::Principal;
use canic_host::response_parse::{find_field, parse_json_u64};
use std::fmt::Write as _;

pub(super) fn parse_issuer_principal(issuer: &str) -> Result<String, AuthCommandError> {
    Principal::from_text(issuer)
        .map(|principal| principal.to_text())
        .map_err(|_| AuthCommandError::InvalidIssuerPrincipal {
            issuer: issuer.to_string(),
        })
}

pub(super) fn parse_renewal_status_summary(output: &str) -> Option<AuthRenewalStatusSummary> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    let payload = find_field(&value, "Ok").unwrap_or(&value);
    let template = find_field(payload, "template").and_then(option_payload);
    let state = find_field(payload, "state").and_then(option_payload);
    let active_attempt = find_field(payload, "active_attempt").and_then(option_payload);

    Some(AuthRenewalStatusSummary {
        template: AuthRenewalTemplateStatus {
            present: template.is_some(),
            enabled: template
                .and_then(|value| find_field(value, "enabled"))
                .and_then(serde_json::Value::as_bool),
            cert_ttl_ns: template
                .and_then(|value| find_field(value, "cert_ttl_ns"))
                .and_then(parse_u64_deep)
                .map(|value| value.to_string()),
        },
        state: AuthRenewalStateStatus {
            present: state.is_some(),
            last_installed_cert_hash: state
                .and_then(|value| find_field(value, "last_installed_cert_hash"))
                .and_then(parse_optional_bytes32_hex),
            last_outcome: state
                .and_then(|value| find_field(value, "last_outcome"))
                .and_then(parse_variant_code),
            consecutive_failures: state
                .and_then(|value| find_field(value, "consecutive_failures"))
                .and_then(parse_u64_deep),
            last_installed_expires_at_ns: state
                .and_then(|value| find_field(value, "last_installed_expires_at_ns"))
                .and_then(parse_optional_u64_deep)
                .map(|value| value.to_string()),
            last_installed_refresh_after_ns: state
                .and_then(|value| find_field(value, "last_installed_refresh_after_ns"))
                .and_then(parse_optional_u64_deep)
                .map(|value| value.to_string()),
            next_attempt_after_ns: state
                .and_then(|value| find_field(value, "next_attempt_after_ns"))
                .and_then(parse_u64_deep)
                .map(|value| value.to_string()),
            active_attempt_id: state
                .and_then(|value| find_field(value, "active_attempt_id"))
                .and_then(parse_optional_bytes32_hex),
        },
        active_attempt: AuthRenewalActiveAttemptStatus {
            present: active_attempt.is_some(),
            status: active_attempt
                .and_then(|value| find_field(value, "status"))
                .and_then(parse_variant_code),
            batch_id: active_attempt
                .and_then(|value| find_field(value, "batch_id"))
                .and_then(parse_bytes32_hex_deep),
            prepared_expires_at_ns: active_attempt
                .and_then(|value| find_field(value, "prepared_expires_at_ns"))
                .and_then(parse_u64_deep)
                .map(|value| value.to_string()),
            failure: active_attempt
                .and_then(|value| find_field(value, "failure"))
                .and_then(parse_optional_variant_code),
        },
    })
}

pub(super) fn parse_issuer_observed_status(output: &str) -> Option<AuthIssuerObservedStatus> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    let payload = find_field(&value, "Ok").unwrap_or(&value);

    Some(AuthIssuerObservedStatus {
        status: find_field(payload, "status").and_then(parse_variant_code)?,
        cert_hash: find_field(payload, "cert_hash").and_then(parse_optional_bytes32_hex),
        expires_at_ns: find_field(payload, "expires_at_ns")
            .and_then(parse_optional_u64_deep)
            .map(|value| value.to_string()),
        refresh_after_ns: find_field(payload, "refresh_after_ns")
            .and_then(parse_optional_u64_deep)
            .map(|value| value.to_string()),
    })
}

fn option_payload(value: &serde_json::Value) -> Option<&serde_json::Value> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::Array(values) => values.first().and_then(option_payload),
        _ => Some(value),
    }
}

fn parse_optional_u64_deep(value: &serde_json::Value) -> Option<u64> {
    option_payload(value).and_then(parse_u64_deep)
}

fn parse_u64_deep(value: &serde_json::Value) -> Option<u64> {
    parse_json_u64(value).or_else(|| match value {
        serde_json::Value::Array(values) => values.iter().find_map(parse_u64_deep),
        serde_json::Value::Object(map) => map.values().find_map(parse_u64_deep),
        _ => None,
    })
}

fn parse_optional_bytes32_hex(value: &serde_json::Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    parse_bytes32_hex_deep(value).or_else(|| match value {
        serde_json::Value::Array(values) if values.len() == 1 => {
            parse_optional_bytes32_hex(&values[0])
        }
        _ => None,
    })
}

fn parse_bytes32_hex_deep(value: &serde_json::Value) -> Option<String> {
    parse_bytes32_json(value).map(|bytes| hex_bytes(&bytes))
}

fn parse_optional_variant_code(value: &serde_json::Value) -> Option<String> {
    option_payload(value).and_then(parse_variant_code)
}

fn parse_variant_code(value: &serde_json::Value) -> Option<String> {
    parse_variant_name(value).map(|variant| pascal_to_snake(&variant))
}

fn parse_variant_name(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Object(map) => map.keys().next().cloned(),
        serde_json::Value::Array(values) => values.iter().find_map(parse_variant_name),
        _ => None,
    }
}

fn pascal_to_snake(value: &str) -> String {
    let mut rendered = String::with_capacity(value.len());
    for (index, ch) in value.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0 {
                rendered.push('_');
            }
            rendered.push(ch.to_ascii_lowercase());
        } else {
            rendered.push(ch);
        }
    }
    rendered
}

fn parse_bytes32_json(value: &serde_json::Value) -> Option<[u8; 32]> {
    match value {
        serde_json::Value::Array(values) => bytes32_from_iter(
            values
                .iter()
                .map(parse_json_byte)
                .collect::<Option<Vec<_>>>()?,
        ),
        serde_json::Value::String(value) => parse_hex_bytes32(value),
        serde_json::Value::Object(map) => map.values().find_map(parse_bytes32_json),
        _ => None,
    }
}

fn parse_json_byte(value: &serde_json::Value) -> Option<u8> {
    let byte = parse_json_u64(value)?;
    u8::try_from(byte).ok()
}

fn bytes32_from_iter(bytes: Vec<u8>) -> Option<[u8; 32]> {
    bytes.try_into().ok()
}

fn parse_hex_bytes32(value: &str) -> Option<[u8; 32]> {
    let hex = value.strip_prefix("0x").unwrap_or(value);
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = [0_u8; 32];
    for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let high = hex_value(chunk[0])?;
        let low = hex_value(chunk[1])?;
        bytes[index] = (high << 4) | low;
    }
    Some(bytes)
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(super) fn root_issuer_renewal_status_arg(issuer_pid: &str) -> String {
    format!(r#"(record {{ issuer_pid = principal "{issuer_pid}" }})"#)
}

pub(super) fn hex_bytes(bytes: &[u8; 32]) -> String {
    let mut rendered = String::with_capacity(64);
    for byte in bytes {
        write!(&mut rendered, "{byte:02x}").expect("write to string");
    }
    rendered
}
