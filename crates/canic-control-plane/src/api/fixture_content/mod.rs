//! Module: api::fixture_content
//!
//! Responsibility: expose the shared host/Store content compiler.
//! Does not own: storage, caller admission or network effects.
//! Boundary: delegates deterministic conversion and validation to ops.

use canic_core::dto::fixture_provisioning::{FixtureDescriptor, FixtureStoreError};

///
/// FixtureContentApi
///
/// Control-plane content compiler shared by host builds and Store admission.
///
pub struct FixtureContentApi;

impl FixtureContentApi {
    /// Validate the full publication envelope and hash release-independent content.
    pub fn content_id(descriptor: &FixtureDescriptor) -> Result<[u8; 32], FixtureStoreError> {
        crate::ops::fixture_content::content_id(descriptor)
    }

    /// Verify a bounded payload before retaining or submitting it.
    pub fn verify_chunk(
        descriptor: &FixtureDescriptor,
        index: u32,
        bytes: &[u8],
    ) -> Result<(), FixtureStoreError> {
        crate::ops::fixture_content::verify_chunk(descriptor, index, bytes)
    }
}
