//! Module: ops::fixture_content
//!
//! Responsibility: validate opaque fixture descriptors and bounded payload identity.
//! Does not own: storage, release selection, endpoint authentication or publication.
//! Boundary: publication and installed consumers share one content identity and chunk verifier.

use crate::{
    CANIC_WASM_CHUNK_BYTES,
    dto::fixture_provisioning::{FixtureDescriptor, FixtureStoreError},
    ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES,
};
use sha2::{Digest, Sha256};

/// Content hashing excludes all release and target identities to avoid circular binding.
pub fn content_id(descriptor: &FixtureDescriptor) -> Result<[u8; 32], FixtureStoreError> {
    if descriptor.schema_version != 1
        || descriptor.chunks.is_empty()
        || descriptor.chunks.len() > DEFAULT_UPDATE_INGRESS_MAX_BYTES / 32
    {
        return Err(FixtureStoreError::Bounds);
    }
    let mut length = 0_u64;
    let mut digest = Sha256::new();
    digest.update(b"canic.fixture.content.v1\0");
    digest.update(descriptor.format_hash);
    digest.update(descriptor.encoded_length.to_be_bytes());
    digest.update(
        u32::try_from(descriptor.chunks.len())
            .map_err(|_| FixtureStoreError::Bounds)?
            .to_be_bytes(),
    );
    for chunk in &descriptor.chunks {
        if chunk.length == 0 || chunk.length as usize > CANIC_WASM_CHUNK_BYTES {
            return Err(FixtureStoreError::Bounds);
        }
        length = length
            .checked_add(u64::from(chunk.length))
            .ok_or(FixtureStoreError::Bounds)?;
        digest.update(chunk.length.to_be_bytes());
        digest.update(chunk.digest);
    }
    if length != descriptor.encoded_length {
        return Err(FixtureStoreError::Content);
    }
    digest.update(descriptor.completion_summary);
    Ok(digest.finalize().into())
}

/// Check one independently decodable chunk against its exact ordered descriptor.
pub fn verify_chunk(
    descriptor: &FixtureDescriptor,
    index: u32,
    bytes: &[u8],
) -> Result<(), FixtureStoreError> {
    let chunk = descriptor
        .chunks
        .get(index as usize)
        .ok_or(FixtureStoreError::Bounds)?;
    if bytes.len() != chunk.length as usize || bytes.len() > CANIC_WASM_CHUNK_BYTES {
        return Err(FixtureStoreError::Bounds);
    }
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    if digest != chunk.digest {
        return Err(FixtureStoreError::Content);
    }
    Ok(())
}
