use super::{ReplicaQueryError, cbor::decode_status_root_key, transport};
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
        .or_else(|| decode_status_root_key(body).ok().flatten())
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

fn nonempty_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
mod tests;
