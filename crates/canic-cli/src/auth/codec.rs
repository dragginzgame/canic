//! Module: canic_cli::auth::codec
//!
//! Responsibility: decode delegated-auth responses and build small Candid arguments.
//! Does not own: command execution, transport, or operator rendering.

use super::{
    AuthCommandError, AuthIssuerObservedStatus, AuthRenewalActiveAttemptStatus,
    AuthRenewalBatchWork, AuthRenewalProvisioner, AuthRenewalStateStatus, AuthRenewalStatusSummary,
    AuthRenewalTemplateStatus,
};
use candid::Principal;
use canic_host::response_parse::{
    candid_record_blocks, field_value_after_equals, find_field, parse_json_u64, parse_u64_digits,
    response_candid,
};
use std::{collections::BTreeSet, fmt::Write as _};

pub(super) fn parse_work_batches(output: &str) -> Option<Vec<AuthRenewalBatchWork>> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(batches) = find_field(&value, "batches").and_then(serde_json::Value::as_array) {
            let parsed = parse_work_batches_json(batches)?;
            return Some(dedupe_work_batches(parsed));
        }
        if let Some(candid) = response_candid(&value) {
            return parse_work_batches_candid(candid);
        }
    }
    parse_work_batches_candid(output)
}

pub(super) fn parse_issuer_principal(issuer: &str) -> Result<String, AuthCommandError> {
    Principal::from_text(issuer)
        .map(|principal| principal.to_text())
        .map_err(|_| AuthCommandError::InvalidIssuerPrincipal {
            issuer: issuer.to_string(),
        })
}

pub(super) fn parse_principal_text(principal: &str) -> Result<String, AuthCommandError> {
    Principal::from_text(principal)
        .map(|principal| principal.to_text())
        .map_err(|_| AuthCommandError::InvalidPrincipal {
            principal: principal.to_string(),
        })
}

pub(super) fn parse_renewal_provisioners(output: &str) -> Option<Vec<AuthRenewalProvisioner>> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(output) {
        let payload = find_field(&value, "Ok").unwrap_or(&value);
        if let Some(values) =
            find_field(payload, "provisioners").and_then(serde_json::Value::as_array)
        {
            let mut provisioners = values
                .iter()
                .map(parse_renewal_provisioner_json)
                .collect::<Option<Vec<_>>>()?;
            sort_provisioners(&mut provisioners);
            return Some(provisioners);
        }
        if let Some(candid) = response_candid(&value) {
            return parse_renewal_provisioners_candid(candid);
        }
    }
    parse_renewal_provisioners_candid(output)
}

pub(super) fn parse_renewal_provisioner_response(output: &str) -> Option<AuthRenewalProvisioner> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(output) {
        let payload = find_field(&value, "Ok").unwrap_or(&value);
        if let Some(provisioner) =
            find_field(payload, "provisioner").and_then(parse_renewal_provisioner_json)
        {
            return Some(provisioner);
        }
        if let Some(candid) = response_candid(&value) {
            return parse_renewal_provisioners_candid(candid)?
                .into_iter()
                .next();
        }
    }
    parse_renewal_provisioners_candid(output)?
        .into_iter()
        .next()
}

fn parse_renewal_provisioner_json(value: &serde_json::Value) -> Option<AuthRenewalProvisioner> {
    Some(AuthRenewalProvisioner {
        principal: find_field(value, "principal").and_then(parse_principal_json)?,
        enabled: find_field(value, "enabled").and_then(serde_json::Value::as_bool)?,
    })
}

fn parse_principal_json(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Principal::from_text(value)
            .ok()
            .map(|principal| principal.to_text()),
        serde_json::Value::Array(values) => values.iter().find_map(parse_principal_json),
        serde_json::Value::Object(map) => map.values().find_map(parse_principal_json),
        _ => None,
    }
}

fn parse_renewal_provisioners_candid(output: &str) -> Option<Vec<AuthRenewalProvisioner>> {
    if !output.contains("principal") || !output.contains("enabled") {
        return None;
    }
    let mut provisioners = candid_record_blocks(output)
        .into_iter()
        .filter(|block| block.contains("principal") && block.contains("enabled"))
        .filter_map(parse_renewal_provisioner_candid)
        .collect::<Vec<_>>();
    sort_provisioners(&mut provisioners);
    provisioners.dedup_by(|left, right| left.principal == right.principal);
    Some(provisioners)
}

fn parse_renewal_provisioner_candid(block: &str) -> Option<AuthRenewalProvisioner> {
    Some(AuthRenewalProvisioner {
        principal: field_value_after_equals(block, "principal").and_then(parse_candid_principal)?,
        enabled: field_value_after_equals(block, "enabled").and_then(parse_candid_bool)?,
    })
}

fn parse_candid_principal(value: &str) -> Option<String> {
    let value = value.trim_start().strip_prefix("principal")?.trim_start();
    let value = value.strip_prefix('"')?;
    let end = value.find('"')?;
    Principal::from_text(&value[..end])
        .ok()
        .map(|principal| principal.to_text())
}

fn parse_candid_bool(value: &str) -> Option<bool> {
    let value = value.trim_start();
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn sort_provisioners(provisioners: &mut [AuthRenewalProvisioner]) {
    provisioners.sort_by(|left, right| left.principal.cmp(&right.principal));
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

fn parse_work_batches_json(values: &[serde_json::Value]) -> Option<Vec<AuthRenewalBatchWork>> {
    values.iter().map(parse_work_batch_json).collect()
}

fn parse_work_batch_json(value: &serde_json::Value) -> Option<AuthRenewalBatchWork> {
    let batch_id = value
        .get("batch_id")
        .or_else(|| find_field(value, "batch_id"))
        .and_then(parse_bytes32_json)?;
    let attempt_count = value
        .get("attempt_count")
        .and_then(parse_json_u64)
        .or_else(|| {
            value
                .get("attempts")
                .and_then(serde_json::Value::as_array)
                .map(|attempts| attempts.len() as u64)
        });
    Some(AuthRenewalBatchWork {
        batch_id,
        attempt_count,
    })
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

fn parse_work_batches_candid(output: &str) -> Option<Vec<AuthRenewalBatchWork>> {
    if !output.contains("batches") {
        return None;
    }
    let batches = candid_record_blocks(output)
        .into_iter()
        .filter(|block| block.contains("batch_id") && block.contains("attempt_count"))
        .filter_map(parse_work_batch_candid)
        .collect::<Vec<_>>();
    Some(dedupe_work_batches(batches))
}

fn parse_work_batch_candid(block: &str) -> Option<AuthRenewalBatchWork> {
    let batch_id = parse_candid_bytes32_field(block, "batch_id")?;
    let attempt_count = field_value_after_equals(block, "attempt_count").and_then(parse_u64_digits);
    Some(AuthRenewalBatchWork {
        batch_id,
        attempt_count,
    })
}

fn parse_candid_bytes32_field(text: &str, field: &str) -> Option<[u8; 32]> {
    let after_eq = field_value_after_equals(text, field)?;
    parse_candid_bytes32(after_eq)
}

fn parse_candid_bytes32(text: &str) -> Option<[u8; 32]> {
    let trimmed = text.trim_start();
    if trimmed.starts_with("blob") {
        return parse_candid_blob_literal(trimmed).and_then(bytes32_from_iter);
    }
    if trimmed.starts_with("vec") {
        return parse_candid_vec_nat8(trimmed).and_then(bytes32_from_iter);
    }
    None
}

fn parse_candid_blob_literal(text: &str) -> Option<Vec<u8>> {
    let after_blob = text.strip_prefix("blob")?.trim_start();
    let bytes = after_blob.as_bytes();
    if bytes.first().copied() != Some(b'"') {
        return None;
    }

    let mut parsed = Vec::new();
    let mut index = 1;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => return Some(parsed),
            b'\\' => {
                if index + 2 < bytes.len()
                    && let (Some(high), Some(low)) =
                        (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
                {
                    parsed.push((high << 4) | low);
                    index += 3;
                    continue;
                }
                let escaped = *bytes.get(index + 1)?;
                parsed.push(match escaped {
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    other => other,
                });
                index += 2;
            }
            byte => {
                parsed.push(byte);
                index += 1;
            }
        }
    }
    None
}

fn parse_candid_vec_nat8(text: &str) -> Option<Vec<u8>> {
    let start = text.find('{')?;
    let end = text[start + 1..].find('}')? + start + 1;
    let body = &text[start + 1..end];
    let mut bytes = Vec::new();
    let mut current = String::new();
    for ch in body.chars() {
        if ch.is_ascii_digit() {
            current.push(ch);
        } else if !current.is_empty() {
            bytes.push(current.parse::<u8>().ok()?);
            current.clear();
        }
    }
    if !current.is_empty() {
        bytes.push(current.parse::<u8>().ok()?);
    }
    Some(bytes)
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

fn dedupe_work_batches(batches: Vec<AuthRenewalBatchWork>) -> Vec<AuthRenewalBatchWork> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for batch in batches {
        if seen.insert(batch.batch_id) {
            deduped.push(batch);
        }
    }
    deduped
}

pub(super) fn root_delegation_renewal_batch_get_arg(batch_id: [u8; 32]) -> String {
    format!("(record {{ batch_id = {} }})", candid_blob32(&batch_id))
}

pub(super) fn root_issuer_renewal_status_arg(issuer_pid: &str) -> String {
    format!(r#"(record {{ issuer_pid = principal "{issuer_pid}" }})"#)
}

pub(super) fn renewal_provisioner_upsert_arg(principal: &str, enabled: bool) -> String {
    format!(r#"(record {{ principal = principal "{principal}"; enabled = {enabled} }})"#)
}

fn candid_blob32(bytes: &[u8; 32]) -> String {
    let mut rendered = String::from("blob \"");
    for byte in bytes {
        write!(&mut rendered, "\\{byte:02x}").expect("write to string");
    }
    rendered.push('"');
    rendered
}

pub(super) fn hex_bytes(bytes: &[u8; 32]) -> String {
    let mut rendered = String::with_capacity(64);
    for byte in bytes {
        write!(&mut rendered, "{byte:02x}").expect("write to string");
    }
    rendered
}
