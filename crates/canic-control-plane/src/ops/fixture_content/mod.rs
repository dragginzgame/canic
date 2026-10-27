//! Module: ops::fixture_content
//!
//! Responsibility: validate opaque fixture descriptors and bounded payload identity.
//! Does not own: storage, release selection, endpoint authentication or publication.
//! Boundary: host compilation and Store admission use the same wire-envelope and hash rules.

use canic_contracts::dto::fixture_provisioning::{FixtureDescriptor, FixtureStoreError};
use canic_contracts::protocol::store_publication;

/// Content hashing excludes all release and target identities to avoid circular binding.

pub fn content_id(descriptor: &FixtureDescriptor) -> Result<[u8; 32], FixtureStoreError> {
    let content_id = canic_core::api::fixture_content::FixtureContentApi::content_id(descriptor)?;
    ic_wire::native::encode_request(
        store_publication::fixture_prepare::METHOD_ID,
        descriptor,
        store_publication::REQUEST_LIMITS,
    )
    .map_err(|_| FixtureStoreError::Bounds)?;
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
