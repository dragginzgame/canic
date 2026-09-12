//! Fixture Store boundary adapters; endpoints authenticate Root commands and exact target reads.

use crate::{ops::fixture_store, workflow::fixture_store as workflow};
use candid::Principal;
use canic_core::dto::{
    error::Error,
    fixture_provisioning::{
        FixtureChunkRead, FixtureChunkUpload, FixtureDescriptor, FixtureGrant, FixtureGrantRequest,
        FixtureSourceStatus, FixtureStoreError,
    },
};

/// Typed Store API for immutable fixture publication and installation-bound delivery.
pub struct FixtureStoreApi;

impl FixtureStoreApi {
    /// Admit an immutable source descriptor under the Store's existing budget.
    pub fn prepare(
        descriptor: FixtureDescriptor,
    ) -> Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> {
        workflow::prepare(descriptor)
    }

    /// Commit one verified chunk and its durable ingestion cursor together.
    pub fn upload(
        request: FixtureChunkUpload,
    ) -> Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> {
        workflow::upload(request)
    }

    /// Apply an authenticated Root's exact compare-and-set target grant intent.
    pub fn set_grant(
        request: FixtureGrantRequest,
    ) -> Result<Result<FixtureGrant, FixtureStoreError>, Error> {
        workflow::set_grant(request)
    }

    /// Observe retained ingestion progress without scanning source payloads.
    pub fn source_status(content: [u8; 32]) -> Result<FixtureSourceStatus, FixtureStoreError> {
        fixture_store::source_status(content)
    }

    /// Observe the current target grant, including its revoked revision.
    #[must_use]
    pub fn grant_status(target: Principal) -> Option<FixtureGrant> {
        fixture_store::grant_status(target)
    }

    /// Authenticate the exact installed target before accessing its selected source.
    pub fn authorize_read(
        caller: Principal,
        request: &FixtureChunkRead,
    ) -> Result<(), FixtureStoreError> {
        fixture_store::authorize_read(caller, request)
    }

    /// Read one source chunk after endpoint authentication, rechecking its revision.
    pub fn read(request: FixtureChunkRead) -> Result<Vec<u8>, FixtureStoreError> {
        fixture_store::read(request)
    }
}
