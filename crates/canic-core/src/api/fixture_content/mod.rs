//! Module: api::fixture_content
//!
//! Responsibility: expose canonical fixture content and chunk validation.
//! Does not own: Store admission, source selection or application data validation.
//! Boundary: publication owners additionally enforce their complete wire envelope.

use crate::dto::fixture_provisioning::{FixtureDescriptor, FixtureStoreError};

/// Canonical opaque content validation shared by publishers and installed consumers.
pub struct FixtureContentApi;

impl FixtureContentApi {
    /// Hash the bounded descriptor independently of its release and installation.
    pub fn content_id(descriptor: &FixtureDescriptor) -> Result<[u8; 32], FixtureStoreError> {
        crate::ops::fixture_content::content_id(descriptor)
    }

    /// Verify exact length, position and digest before application delivery.
    pub fn verify_chunk(
        descriptor: &FixtureDescriptor,
        index: u32,
        bytes: &[u8],
    ) -> Result<(), FixtureStoreError> {
        crate::ops::fixture_content::verify_chunk(descriptor, index, bytes)
    }
}
