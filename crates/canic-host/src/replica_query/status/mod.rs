use super::{ReplicaQueryError, transport};
use std::path::Path;

#[must_use]
pub fn local_replica_status_reachable_from_root(network: Option<&str>, icp_root: &Path) -> bool {
    transport::get_http_status(&transport::local_replica_endpoint_from_root(
        network, icp_root,
    ))
    .is_ok()
}

pub fn local_replica_root_key_from_root(
    network: Option<&str>,
    icp_root: &Path,
) -> Result<Option<String>, ReplicaQueryError> {
    let endpoint = transport::local_replica_endpoint_from_root(network, icp_root);
    let body = transport::get_http_status(&endpoint)?;
    Ok(parse_local_replica_root_key(&body))
}

pub(super) fn parse_local_replica_root_key(body: &[u8]) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|value| root_key_from_json(&value))
        .or_else(|| {
            serde_cbor::from_slice::<serde_cbor::Value>(body)
                .ok()
                .and_then(|value| root_key_from_cbor(&value))
        })
}

fn root_key_from_json(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => nonempty_text(text),
        serde_json::Value::Array(values) => values.iter().find_map(root_key_from_json),
        serde_json::Value::Object(map) => map
            .get("root_key")
            .and_then(root_key_from_json)
            .or_else(|| map.values().find_map(root_key_from_json)),
        _ => None,
    }
}

fn root_key_from_cbor(value: &serde_cbor::Value) -> Option<String> {
    match value {
        serde_cbor::Value::Bytes(bytes) => (!bytes.is_empty()).then(|| hex_bytes(bytes)),
        serde_cbor::Value::Text(text) => nonempty_text(text),
        serde_cbor::Value::Array(values) => values.iter().find_map(root_key_from_cbor),
        serde_cbor::Value::Map(map) => map
            .iter()
            .find_map(|(key, value)| match key {
                serde_cbor::Value::Text(key) if key == "root_key" => root_key_from_cbor(value),
                _ => None,
            })
            .or_else(|| map.values().find_map(root_key_from_cbor)),
        _ => None,
    }
}

fn nonempty_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(test)]
mod tests;
