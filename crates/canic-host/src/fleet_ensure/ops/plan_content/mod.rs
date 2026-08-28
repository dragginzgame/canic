//! Module: fleet_ensure::ops::plan_content
//!
//! Responsibility: retain and resolve content-addressed Store chunks for one current plan.
//! Does not own: Store policy, action ordering, or remote publication.
//! Boundary: durable plan JSON retains chunk hashes, never inline chunk bytes.

#[cfg(test)]
mod tests;

use super::{EnsurePaths, EnsureStateError};
use crate::{
    durable_io::{
        BoundedRegularFileReadError, RegularFileReadError, create_new_bytes_with_parents,
        read_optional_regular_bytes_bounded,
    },
    fleet_ensure::model::{CurrentFleetProtocolAction, EnsureAction, FleetEnsurePlan},
};
use canic_core::cdk::utils::hash::{decode_hex, hex_bytes, wasm_hash};
use serde_json::{Map, Value};
use std::{collections::BTreeMap, io};

type ChunkSetKey = (String, String);

pub(super) fn retain(paths: &EnsurePaths, plan: &FleetEnsurePlan) -> Result<(), EnsureStateError> {
    let authorities = typed_chunk_authorities(plan)?;
    for action in &plan.protocol_actions {
        let EnsureAction::FleetProtocol { action, .. } = action else {
            continue;
        };
        let CurrentFleetProtocolAction::PublishStoreChunk { request } = action.as_ref() else {
            continue;
        };
        let key = chunk_set_key(request.template_id.as_str(), request.version.as_str());
        let expected = wasm_hash(&request.bytes);
        if let Some(prepared) = prepared_chunk(&authorities, &key, request.chunk_index)?
            && prepared != expected
        {
            return authority_error("published chunk differs from its prepared hash");
        }
        validate_chunk(&request.bytes, &expected, request.bytes.len() as u64)?;
        retain_object(paths, &expected, &request.bytes)?;
    }
    Ok(())
}

pub(super) fn remove_inline_bytes(projection: &mut Value) -> Result<(), EnsureStateError> {
    for action in protocol_actions_mut(projection)? {
        if fleet_protocol_action_kind(action)? != Some("publish_store_chunk") {
            continue;
        }
        let request = request_mut(action)?;
        let Some(bytes) = request.remove("bytes") else {
            return authority_error("published chunk projection has no inline bytes");
        };
        let bytes = serde_json::from_value::<Vec<u8>>(bytes)
            .map_err(|_| authority("published chunk projection bytes are invalid"))?;
        let hash = wasm_hash(&bytes);
        request.insert("bytes_sha256".to_string(), Value::String(hex_bytes(&hash)));
        request.insert(
            "bytes_size".to_string(),
            Value::from(u64::try_from(bytes.len()).unwrap_or(u64::MAX)),
        );
    }
    Ok(())
}

pub(super) fn hydrate(paths: &EnsurePaths, projection: &mut Value) -> Result<(), EnsureStateError> {
    let authorities = projected_chunk_authorities(projection)?;
    for action in protocol_actions_mut(projection)? {
        if fleet_protocol_action_kind(action)? != Some("publish_store_chunk") {
            continue;
        }
        let request = request_mut(action)?;
        let template_id = text_field(request, "template_id")?;
        let version = text_field(request, "version")?;
        let chunk_index = u32::try_from(unsigned_field(request, "chunk_index")?)
            .map_err(|_| authority("published chunk index exceeds u32"))?;
        let key = chunk_set_key(&template_id, &version);
        let (bytes, expected, expected_size) = if let Some(inline) = request.get("bytes") {
            if request.contains_key("bytes_sha256") || request.contains_key("bytes_size") {
                return authority_error("published chunk mixes inline and referenced content");
            }
            let bytes = serde_json::from_value::<Vec<u8>>(inline.clone())
                .map_err(|_| authority("published inline chunk bytes are invalid"))?;
            let expected = wasm_hash(&bytes);
            let expected_size = bytes.len() as u64;
            (bytes, expected, expected_size)
        } else {
            let expected = decode_chunk_hash(request)?;
            let expected_size = unsigned_field(request, "bytes_size")?;
            let bytes = read_object(paths, &expected, expected_size)?;
            (bytes, expected, expected_size)
        };
        if let Some(prepared) = prepared_chunk(&authorities, &key, chunk_index)?
            && prepared != expected
        {
            return authority_error("published chunk differs from its prepared hash");
        }
        validate_chunk(&bytes, &expected, expected_size)?;
        request.remove("bytes_sha256");
        request.remove("bytes_size");
        request.insert(
            "bytes".to_string(),
            serde_json::to_value(bytes)
                .map_err(|_| authority("published chunk bytes cannot be projected"))?,
        );
    }
    Ok(())
}

fn typed_chunk_authorities(
    plan: &FleetEnsurePlan,
) -> Result<BTreeMap<ChunkSetKey, Vec<Vec<u8>>>, EnsureStateError> {
    let mut authorities = BTreeMap::new();
    for action in &plan.protocol_actions {
        let EnsureAction::FleetProtocol { action, .. } = action else {
            continue;
        };
        let CurrentFleetProtocolAction::PrepareStoreChunkSet { request } = action.as_ref() else {
            continue;
        };
        insert_authority(
            &mut authorities,
            chunk_set_key(request.template_id.as_str(), request.version.as_str()),
            request.chunk_hashes.clone(),
        )?;
    }
    Ok(authorities)
}

fn projected_chunk_authorities(
    projection: &Value,
) -> Result<BTreeMap<ChunkSetKey, Vec<Vec<u8>>>, EnsureStateError> {
    let mut authorities = BTreeMap::new();
    for action in protocol_actions(projection)? {
        if fleet_protocol_action_kind(action)? != Some("prepare_store_chunk_set") {
            continue;
        }
        let request = request(action)?;
        let chunk_hashes = serde_json::from_value::<Vec<Vec<u8>>>(
            request
                .get("chunk_hashes")
                .cloned()
                .ok_or_else(|| authority("prepared chunk authority has no hashes"))?,
        )
        .map_err(|_| authority("prepared chunk hashes are invalid"))?;
        insert_authority(
            &mut authorities,
            chunk_set_key(
                &text_field(request, "template_id")?,
                &text_field(request, "version")?,
            ),
            chunk_hashes,
        )?;
    }
    Ok(authorities)
}

fn insert_authority(
    authorities: &mut BTreeMap<ChunkSetKey, Vec<Vec<u8>>>,
    key: ChunkSetKey,
    hashes: Vec<Vec<u8>>,
) -> Result<(), EnsureStateError> {
    if hashes.is_empty() || hashes.iter().any(|hash| hash.len() != 32) {
        return authority_error("prepared chunk authority contains invalid hashes");
    }
    if let Some(existing) = authorities.insert(key, hashes.clone())
        && existing != hashes
    {
        return authority_error("one Store payload has conflicting prepared chunk authority");
    }
    Ok(())
}

fn prepared_chunk<'a>(
    authorities: &'a BTreeMap<ChunkSetKey, Vec<Vec<u8>>>,
    key: &ChunkSetKey,
    index: u32,
) -> Result<Option<&'a [u8]>, EnsureStateError> {
    let Some(hashes) = authorities.get(key) else {
        return Ok(None);
    };
    hashes
        .get(index as usize)
        .map(Vec::as_slice)
        .map(Some)
        .ok_or_else(|| authority("published chunk index exceeds its prepared authority"))
}

fn validate_chunk(
    bytes: &[u8],
    expected: &[u8],
    expected_size: u64,
) -> Result<(), EnsureStateError> {
    if bytes.is_empty()
        || bytes.len() > canic_core::CANIC_WASM_CHUNK_BYTES
        || bytes.len() as u64 != expected_size
        || wasm_hash(bytes) != expected
    {
        return authority_error("published chunk differs from its retained hash or size");
    }
    Ok(())
}

fn retain_object(
    paths: &EnsurePaths,
    expected: &[u8],
    bytes: &[u8],
) -> Result<(), EnsureStateError> {
    let path = object_path(paths, expected);
    match create_new_bytes_with_parents(&path, bytes) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            let retained = read_object(paths, expected, bytes.len() as u64)?;
            if retained == bytes {
                Ok(())
            } else {
                Err(EnsureStateError::StoreChunkMismatch { path })
            }
        }
        Err(source) => Err(EnsureStateError::Io { path, source }),
    }
}

fn read_object(
    paths: &EnsurePaths,
    expected: &[u8],
    expected_size: u64,
) -> Result<Vec<u8>, EnsureStateError> {
    let path = object_path(paths, expected);
    if expected_size == 0
        || expected_size > u64::try_from(canic_core::CANIC_WASM_CHUNK_BYTES).unwrap_or(u64::MAX)
    {
        return Err(EnsureStateError::StoreChunkMismatch { path });
    }
    let maximum_bytes = usize::try_from(expected_size)
        .map_err(|_| EnsureStateError::StoreChunkMismatch { path: path.clone() })?;
    let bytes = match read_optional_regular_bytes_bounded(&path, maximum_bytes) {
        Ok(Some(bytes)) => bytes,
        Ok(None) | Err(BoundedRegularFileReadError::Read(RegularFileReadError::NotRegular)) => {
            return Err(EnsureStateError::StoreChunkUnavailable { path });
        }
        Err(BoundedRegularFileReadError::TooLarge) => {
            return Err(EnsureStateError::StoreChunkMismatch { path });
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::Io(source))) => {
            return Err(EnsureStateError::Io { path, source });
        }
        #[cfg(not(unix))]
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::UnsupportedPlatform)) => {
            return Err(EnsureStateError::StoreChunkUnavailable { path });
        }
    };
    if validate_chunk(&bytes, expected, expected_size).is_err() {
        return Err(EnsureStateError::StoreChunkMismatch { path });
    }
    Ok(bytes)
}

fn object_path(paths: &EnsurePaths, expected: &[u8]) -> std::path::PathBuf {
    paths.content.join(hex_bytes(expected))
}

fn chunk_set_key(template_id: &str, version: &str) -> ChunkSetKey {
    (template_id.to_string(), version.to_string())
}

fn decode_chunk_hash(request: &Map<String, Value>) -> Result<Vec<u8>, EnsureStateError> {
    let encoded = text_field(request, "bytes_sha256")?;
    let decoded = decode_hex(&encoded)
        .map_err(|_| authority("published chunk SHA-256 is not exact lowercase hexadecimal"))?;
    if decoded.len() != 32 || hex_bytes(&decoded) != encoded {
        return authority_error("published chunk SHA-256 is not exact lowercase hexadecimal");
    }
    Ok(decoded)
}

fn protocol_actions(value: &Value) -> Result<&Vec<Value>, EnsureStateError> {
    value
        .get("protocol_actions")
        .and_then(Value::as_array)
        .ok_or_else(|| authority("plan protocol actions are not an array"))
}

fn protocol_actions_mut(value: &mut Value) -> Result<&mut Vec<Value>, EnsureStateError> {
    value
        .get_mut("protocol_actions")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| authority("plan protocol actions are not an array"))
}

fn fleet_protocol_action_kind(action: &Value) -> Result<Option<&str>, EnsureStateError> {
    let outer_kind = action
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| authority("protocol action has no exact kind"))?;
    if outer_kind != "fleet_protocol" {
        return Ok(None);
    }
    action
        .get("action")
        .and_then(|value| value.get("kind"))
        .and_then(Value::as_str)
        .map(Some)
        .ok_or_else(|| authority("Fleet protocol action has no exact kind"))
}

fn request(action: &Value) -> Result<&Map<String, Value>, EnsureStateError> {
    action
        .get("action")
        .and_then(|value| value.get("request"))
        .and_then(Value::as_object)
        .ok_or_else(|| authority("Store action has no exact request"))
}

fn request_mut(action: &mut Value) -> Result<&mut Map<String, Value>, EnsureStateError> {
    action
        .get_mut("action")
        .and_then(|value| value.get_mut("request"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| authority("Store action has no exact request"))
}

fn text_field(
    request: &Map<String, Value>,
    field: &'static str,
) -> Result<String, EnsureStateError> {
    request
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| authority("Store action text authority is invalid"))
}

fn unsigned_field(
    request: &Map<String, Value>,
    field: &'static str,
) -> Result<u64, EnsureStateError> {
    request
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| authority("Store action integer authority is invalid"))
}

fn authority(reason: &'static str) -> EnsureStateError {
    EnsureStateError::StoreChunkAuthority {
        reason: reason.to_string(),
    }
}

fn authority_error<T>(reason: &'static str) -> Result<T, EnsureStateError> {
    Err(authority(reason))
}
