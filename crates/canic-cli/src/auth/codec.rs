//! Module: canic_cli::auth::codec
//!
//! Responsibility: decode delegated-auth responses and build small Candid arguments.
//! Does not own: command execution, transport, or operator rendering.

use super::{
    AuthCommandError, AuthIssuerObservedStatus, AuthRenewalBatchStatus, AuthRenewalStateStatus,
    AuthRenewalStatusSummary, AuthRenewalTemplateStatus,
};
use candid::Principal;
use canic_host::response_parse::{find_field, parse_json_u64};
use std::{error::Error, fmt, fmt::Write as _};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AuthResponseKind {
    RenewalStatus,
    IssuerStatus,
}

impl AuthResponseKind {
    const fn label(self) -> &'static str {
        match self {
            Self::RenewalStatus => "renewal status",
            Self::IssuerStatus => "issuer status",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum AuthResponseParseError {
    InvalidJson {
        kind: AuthResponseKind,
        error: String,
    },
    InvalidPayload(AuthResponseKind),
    InvalidField {
        kind: AuthResponseKind,
        field: &'static str,
    },
}

impl fmt::Display for AuthResponseParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson { kind, error } => {
                write!(
                    formatter,
                    "{} response has invalid JSON: {error}",
                    kind.label()
                )
            }
            Self::InvalidPayload(kind) => {
                write!(
                    formatter,
                    "{} response has an invalid payload",
                    kind.label()
                )
            }
            Self::InvalidField { kind, field } => {
                write!(formatter, "{} response has invalid `{field}`", kind.label())
            }
        }
    }
}

impl Error for AuthResponseParseError {}

pub(super) fn parse_issuer_principal(issuer: &str) -> Result<String, AuthCommandError> {
    Principal::from_text(issuer)
        .map(|principal| principal.to_text())
        .map_err(|_| AuthCommandError::InvalidIssuerPrincipal {
            issuer: issuer.to_string(),
        })
}

pub(super) fn parse_renewal_status_summary(
    output: &str,
) -> Result<AuthRenewalStatusSummary, AuthResponseParseError> {
    let kind = AuthResponseKind::RenewalStatus;
    let value = parse_json_response(output, kind)?;
    let payload = find_field(&value, "Ok").unwrap_or(&value);
    if !payload.is_object() {
        return Err(AuthResponseParseError::InvalidPayload(kind));
    }
    let template = parse_optional_record(payload, kind, "template")?;
    let state = parse_optional_record(payload, kind, "state")?;
    let latest_batch = parse_optional_record(payload, kind, "latest_batch")?;

    Ok(AuthRenewalStatusSummary {
        template: parse_template_status(template)?,
        state: parse_state_status(state)?,
        latest_batch: parse_batch_status(latest_batch)?,
    })
}

fn parse_template_status(
    template: Option<&serde_json::Value>,
) -> Result<AuthRenewalTemplateStatus, AuthResponseParseError> {
    let kind = AuthResponseKind::RenewalStatus;
    Ok(AuthRenewalTemplateStatus {
        present: template.is_some(),
        enabled: parse_nested_optional_field(
            template,
            kind,
            "enabled",
            "template.enabled",
            serde_json::Value::as_bool,
        )?,
        cert_ttl_ns: parse_nested_optional_field(
            template,
            kind,
            "cert_ttl_ns",
            "template.cert_ttl_ns",
            parse_u64_deep,
        )?
        .map(|value| value.to_string()),
    })
}

fn parse_state_status(
    state: Option<&serde_json::Value>,
) -> Result<AuthRenewalStateStatus, AuthResponseParseError> {
    let kind = AuthResponseKind::RenewalStatus;
    Ok(AuthRenewalStateStatus {
        present: state.is_some(),
        last_installed_cert_hash: parse_nested_optional_field(
            state,
            kind,
            "last_installed_cert_hash",
            "state.last_installed_cert_hash",
            parse_optional_bytes32_hex,
        )?,
        last_installed_expires_at_ns: parse_nested_optional_field(
            state,
            kind,
            "last_installed_expires_at_ns",
            "state.last_installed_expires_at_ns",
            parse_optional_u64_deep,
        )?
        .map(|value| value.to_string()),
        last_installed_refresh_after_ns: parse_nested_optional_field(
            state,
            kind,
            "last_installed_refresh_after_ns",
            "state.last_installed_refresh_after_ns",
            parse_optional_u64_deep,
        )?
        .map(|value| value.to_string()),
        next_attempt_after_ns: parse_nested_optional_field(
            state,
            kind,
            "next_attempt_after_ns",
            "state.next_attempt_after_ns",
            parse_u64_deep,
        )?
        .map(|value| value.to_string()),
    })
}

fn parse_batch_status(
    latest_batch: Option<&serde_json::Value>,
) -> Result<AuthRenewalBatchStatus, AuthResponseParseError> {
    let kind = AuthResponseKind::RenewalStatus;
    Ok(AuthRenewalBatchStatus {
        present: latest_batch.is_some(),
        status: parse_nested_optional_field(
            latest_batch,
            kind,
            "status",
            "latest_batch.status",
            parse_variant_code,
        )?,
        batch_id: parse_nested_optional_field(
            latest_batch,
            kind,
            "batch_id",
            "latest_batch.batch_id",
            parse_bytes32_hex_deep,
        )?,
        cert_hash: parse_nested_optional_field(
            latest_batch,
            kind,
            "cert_hash",
            "latest_batch.cert_hash",
            parse_bytes32_hex_deep,
        )?,
        proof_epoch: parse_nested_optional_field(
            latest_batch,
            kind,
            "proof_epoch",
            "latest_batch.proof_epoch",
            parse_u64_deep,
        )?,
        expires_at_ns: parse_nested_optional_field(
            latest_batch,
            kind,
            "expires_at_ns",
            "latest_batch.expires_at_ns",
            parse_u64_deep,
        )?
        .map(|value| value.to_string()),
        installed_at_ns: parse_nested_optional_field(
            latest_batch,
            kind,
            "installed_at_ns",
            "latest_batch.installed_at_ns",
            parse_optional_u64_deep,
        )?
        .map(|value| value.to_string()),
        retry_after_ns: parse_nested_optional_field(
            latest_batch,
            kind,
            "retry_after_ns",
            "latest_batch.retry_after_ns",
            parse_optional_u64_deep,
        )?
        .map(|value| value.to_string()),
        failure: parse_nested_optional_field(
            latest_batch,
            kind,
            "failure",
            "latest_batch.failure",
            |value| {
                option_payload(value)
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            },
        )?,
    })
}

pub(super) fn parse_issuer_observed_status(
    output: &str,
) -> Result<AuthIssuerObservedStatus, AuthResponseParseError> {
    let kind = AuthResponseKind::IssuerStatus;
    let value = parse_json_response(output, kind)?;
    let payload = find_field(&value, "Ok").unwrap_or(&value);
    if !payload.is_object() {
        return Err(AuthResponseParseError::InvalidPayload(kind));
    }

    Ok(AuthIssuerObservedStatus {
        status: parse_required_field(payload, kind, "status", parse_variant_code)?,
        cert_hash: parse_optional_field(
            find_field(payload, "cert_hash"),
            kind,
            "cert_hash",
            parse_optional_bytes32_hex,
        )?,
        expires_at_ns: parse_optional_field(
            find_field(payload, "expires_at_ns"),
            kind,
            "expires_at_ns",
            parse_optional_u64_deep,
        )?
        .map(|value| value.to_string()),
        refresh_after_ns: parse_optional_field(
            find_field(payload, "refresh_after_ns"),
            kind,
            "refresh_after_ns",
            parse_optional_u64_deep,
        )?
        .map(|value| value.to_string()),
    })
}

fn parse_json_response(
    output: &str,
    kind: AuthResponseKind,
) -> Result<serde_json::Value, AuthResponseParseError> {
    serde_json::from_str(output).map_err(|error| AuthResponseParseError::InvalidJson {
        kind,
        error: error.to_string(),
    })
}

fn parse_optional_record<'a>(
    payload: &'a serde_json::Value,
    kind: AuthResponseKind,
    field: &'static str,
) -> Result<Option<&'a serde_json::Value>, AuthResponseParseError> {
    let Some(value) = find_field(payload, field) else {
        return Ok(None);
    };
    let Some(value) = option_payload(value) else {
        return Ok(None);
    };
    if !value.is_object() {
        return Err(AuthResponseParseError::InvalidField { kind, field });
    }
    Ok(Some(value))
}

fn parse_nested_optional_field<T>(
    record: Option<&serde_json::Value>,
    kind: AuthResponseKind,
    field: &'static str,
    path: &'static str,
    parse: impl FnOnce(&serde_json::Value) -> Option<T>,
) -> Result<Option<T>, AuthResponseParseError> {
    parse_optional_field(
        record.and_then(|value| find_field(value, field)),
        kind,
        path,
        parse,
    )
}

fn parse_required_field<T>(
    payload: &serde_json::Value,
    kind: AuthResponseKind,
    field: &'static str,
    parse: impl FnOnce(&serde_json::Value) -> Option<T>,
) -> Result<T, AuthResponseParseError> {
    find_field(payload, field)
        .and_then(parse)
        .ok_or(AuthResponseParseError::InvalidField { kind, field })
}

fn parse_optional_field<T>(
    value: Option<&serde_json::Value>,
    kind: AuthResponseKind,
    field: &'static str,
    parse: impl FnOnce(&serde_json::Value) -> Option<T>,
) -> Result<Option<T>, AuthResponseParseError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || matches!(value, serde_json::Value::Array(values) if values.is_empty()) {
        return Ok(None);
    }
    parse(value)
        .map(Some)
        .ok_or(AuthResponseParseError::InvalidField { kind, field })
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
