//! Module: fleet_ensure::ops::reinstall::adoption::content
//!
//! Responsibility: retain exact publication objects referenced by archived source documents.
//! Does not own: source receipt admission, protocol decoding or remote publication.
//! Boundary: archive recovery resolves content from the archive, never mutable workspace objects.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::read_regular_bytes,
    fleet_ensure::{
        model::MAX_FLEET_ENSURE_PROTOCOL_STEPS,
        ops::{EnsurePaths, EnsureStateError, is_sha256},
    },
};
use canic_core::cdk::utils::hash::sha256_hex;
use serde_json::Value;
use std::collections::BTreeMap;

/// Archive referenced objects before committing local handoff intent.
pub(super) fn retain(paths: &EnsurePaths, document: &[u8]) -> Result<(), EnsureStateError> {
    for (digest, size) in references(document)? {
        let path = paths.content.join(&digest);
        let bytes =
            read_regular_bytes(&path, size).map_err(|error| super::io_error(&path, error))?;
        verify_bytes(&bytes, &digest, size)?;
        super::retain(paths, &digest, &bytes)?;
    }
    Ok(())
}

/// Verify retained objects even when their shared workspace copies are unavailable.
pub(super) fn verify(paths: &EnsurePaths, document: &[u8]) -> Result<(), EnsureStateError> {
    for (digest, size) in references(document)? {
        let path = super::object_path(paths, &digest);
        let bytes =
            read_regular_bytes(&path, size).map_err(|error| super::io_error(&path, error))?;
        verify_bytes(&bytes, &digest, size)?;
    }
    Ok(())
}

fn verify_bytes(bytes: &[u8], digest: &str, size: usize) -> Result<(), EnsureStateError> {
    if bytes.len() != size || sha256_hex(bytes) != digest {
        return Err(super::conflict());
    }
    Ok(())
}

/// Extract only durable content references, without reconstructing an executable action.
fn references(document: &[u8]) -> Result<BTreeMap<String, usize>, EnsureStateError> {
    let value: Value = serde_json::from_slice(document).map_err(|_| super::conflict())?;
    let actions = value
        .get("protocol_actions")
        .and_then(Value::as_array)
        .ok_or_else(super::conflict)?;
    if actions.len() > MAX_FLEET_ENSURE_PROTOCOL_STEPS {
        return Err(super::conflict());
    }
    let mut references = BTreeMap::new();
    for action in actions {
        if action.get("kind").and_then(Value::as_str) != Some("fleet_protocol") {
            continue;
        }
        let publication = action.get("action").ok_or_else(super::conflict)?;
        if !matches!(
            publication.get("kind").and_then(Value::as_str),
            Some("publish_store_chunk" | "publish_store_fixture_chunk")
        ) {
            continue;
        }
        let request = publication.get("request").ok_or_else(super::conflict)?;
        if request.get("bytes").is_some() {
            // Inline bytes are already retained by the document's exact byte hash.
            if request.get("bytes_sha256").is_some() || request.get("bytes_size").is_some() {
                return Err(super::conflict());
            }
            continue;
        }
        let digest = request
            .get("bytes_sha256")
            .and_then(Value::as_str)
            .ok_or_else(super::conflict)?;
        let size = request
            .get("bytes_size")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(super::conflict)?;
        if !is_sha256(digest) || size == 0 || size > canic_core::CANIC_WASM_CHUNK_BYTES {
            return Err(super::conflict());
        }
        if let Some(previous) = references.insert(digest.to_string(), size)
            && previous != size
        {
            return Err(super::conflict());
        }
    }
    Ok(references)
}
