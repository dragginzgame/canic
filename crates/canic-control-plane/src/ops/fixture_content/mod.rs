//! Module: ops::fixture_content
//!
//! Responsibility: validate opaque fixture descriptors and bounded payload identity.
//! Does not own: storage, release selection, endpoint authentication or publication.
//! Boundary: host compilation and Store admission use the same wire-envelope and hash rules.

use crate::dto::template::StoreCommand;
use canic_core::{
    dto::fixture_provisioning::{FixtureDescriptor, FixtureStoreError},
    ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES,
};

/// Content hashing excludes all release and target identities to avoid circular binding.
pub fn content_id(descriptor: &FixtureDescriptor) -> Result<[u8; 32], FixtureStoreError> {
    let content_id = canic_core::api::fixture_content::FixtureContentApi::content_id(descriptor)?;
    let encoded = candid::encode_one(StoreCommand::PrepareFixture(descriptor.clone()))
        .map_err(|_| FixtureStoreError::Bounds)?;
    if encoded.len() > DEFAULT_UPDATE_INGRESS_MAX_BYTES {
        return Err(FixtureStoreError::Bounds);
    }
    Ok(content_id)
}

/// Check one independently decodable chunk against its exact ordered descriptor.
pub fn verify_chunk(
    descriptor: &FixtureDescriptor,
    index: u32,
    bytes: &[u8],
) -> Result<(), FixtureStoreError> {
    canic_core::api::fixture_content::FixtureContentApi::verify_chunk(descriptor, index, bytes)
}
