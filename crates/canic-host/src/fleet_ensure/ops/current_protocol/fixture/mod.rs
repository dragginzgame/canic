//! Module: fleet_ensure::ops::current_protocol::fixture
//!
//! Responsibility: compile and reconcile bounded fixture publication effects.
//! Does not own: the durable journal, target grants or application readiness.
//! Boundary: Root selects descriptors; the exact Store receives retained chunk bytes.

#[cfg(test)]
mod tests;

use super::{CurrentProtocolError, ResolvedProtocolAction, observation, unavailable_observation};
use crate::{
    canister_protocol::{call_wire, query_wire_cli},
    fleet_ensure::{model::CurrentFleetProtocolAction, ops::EffectObservation},
    icp::IcpCli,
    release_set::{
        FleetSubnetRootReleaseSetManifest,
        fixture::{FixtureArtifactManifest, read_chunk},
    },
};
use canic_contracts::{
    dto::{
        fixture_provisioning::{FixtureChunkUpload, FixtureSourceStatus, FixtureStoreError},
        root_store::{RootStoreBootstrapRequest, RootStoreFixture, RootStoreFixturePrepareRequest},
    },
    protocol,
};
use canic_control_plane::api::fixture_content::FixtureContentApi;
use ic_platform::principal::ExternalPrincipal as Principal;
use std::{collections::BTreeSet, path::Path};

pub(super) fn append_actions(
    root: &Path,
    manifest: &FleetSubnetRootReleaseSetManifest,
    fixtures: &FixtureArtifactManifest,
    store: Principal,
    bootstrap: &RootStoreBootstrapRequest,
    actions: &mut Vec<CurrentFleetProtocolAction>,
) -> Result<(), CurrentProtocolError> {
    let mut contents = BTreeSet::new();
    for source in &manifest.fixtures {
        if !contents.insert(source.content_id) {
            continue;
        }
        let entry = fixtures
            .entries
            .iter()
            .find(|entry| entry.role == source.role)
            .ok_or(CurrentProtocolError::ResponseMismatch)?;
        actions.push(CurrentFleetProtocolAction::PrepareStoreFixture {
            maximum_attempts: 1,
            request: RootStoreFixturePrepareRequest {
                bootstrap: bootstrap.clone(),
                role: source.role.clone(),
            },
            source: source.clone(),
            store,
        });
        let count = u32::try_from(source.descriptor.chunks.len())
            .map_err(|_| CurrentProtocolError::ResponseMismatch)?;
        let mut received_bytes = 0;
        for (index, chunk) in source.descriptor.chunks.iter().enumerate() {
            let index = u32::try_from(index).map_err(|_| CurrentProtocolError::ResponseMismatch)?;
            let bytes = read_chunk(root, manifest.release_build_id, entry, index)
                .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
            received_bytes += u64::from(chunk.length);
            actions.push(CurrentFleetProtocolAction::PublishStoreFixtureChunk {
                maximum_attempts: 1,
                request: FixtureChunkUpload {
                    content_id: source.content_id,
                    index,
                    bytes,
                },
                expected: FixtureSourceStatus {
                    content_id: source.content_id,
                    next_chunk: index + 1,
                    chunk_count: count,
                    received_bytes,
                    complete: index + 1 == count,
                },
                source_bytes: source.descriptor.encoded_length,
            });
        }
    }
    Ok(())
}

pub(super) fn observe_preparation(
    icp: &IcpCli,
    resolved: &ResolvedProtocolAction<'_>,
    source: &RootStoreFixture,
) -> Result<EffectObservation, CurrentProtocolError> {
    let status: Result<FixtureSourceStatus, FixtureStoreError> = query_wire_cli(
        icp,
        resolved.target,
        protocol::CANIC_ROOT_FIXTURE_STATUS,
        protocol::root_store_reporting::fixture::METHOD_ID,
        &source.content_id,
        protocol::root_store_reporting::REQUEST_LIMITS,
        protocol::root_store_reporting::RESPONSE_LIMITS,
    )?;
    match status {
        Ok(status) => {
            verify_source_status(source, &status)?;
            observation(true, &status)
        }
        Err(FixtureStoreError::NotFound) => Ok(unavailable_observation()),
        Err(error) => Err(CurrentProtocolError::Fixture(error)),
    }
}

pub(super) fn observe_upload(
    icp: &IcpCli,
    resolved: &ResolvedProtocolAction<'_>,
    expected: &FixtureSourceStatus,
    source_bytes: u64,
) -> Result<EffectObservation, CurrentProtocolError> {
    let status: Result<FixtureSourceStatus, FixtureStoreError> = query_wire_cli(
        icp,
        resolved.target,
        protocol::store_catalog::fixture::METHOD,
        protocol::store_catalog::fixture::METHOD_ID,
        &expected.content_id,
        protocol::store_catalog::REQUEST_LIMITS,
        protocol::store_catalog::RESPONSE_LIMITS,
    )?;
    match status {
        Ok(status) => observation(upload_applied(expected, source_bytes, &status)?, &status),
        Err(FixtureStoreError::NotFound) => Ok(unavailable_observation()),
        Err(error) => Err(CurrentProtocolError::Fixture(error)),
    }
}

pub(super) fn prepare(
    icp: &IcpCli,
    resolved: &ResolvedProtocolAction<'_>,
    request: &RootStoreFixturePrepareRequest,
    source: &RootStoreFixture,
) -> Result<Vec<u8>, CurrentProtocolError> {
    if request.role != source.role {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    let status: Result<FixtureSourceStatus, FixtureStoreError> = call_wire(
        icp,
        resolved.target,
        protocol::root_store_control::prepare::METHOD,
        protocol::root_store_control::prepare::METHOD_ID,
        request,
        protocol::root_store_control::REQUEST_LIMITS,
        protocol::root_store_control::RESPONSE_LIMITS,
    )?;
    let status = status.map_err(CurrentProtocolError::Fixture)?;
    verify_source_status(source, &status)?;
    Ok(source.content_id.to_vec())
}

pub(super) fn upload(
    icp: &IcpCli,
    resolved: &ResolvedProtocolAction<'_>,
    request: &FixtureChunkUpload,
    expected: &FixtureSourceStatus,
    source_bytes: u64,
) -> Result<Vec<u8>, CurrentProtocolError> {
    let status: Result<FixtureSourceStatus, FixtureStoreError> = call_wire(
        icp,
        resolved.target,
        protocol::CANIC_WASM_STORE_PUBLISH_FIXTURE,
        protocol::fixture_bytes::upload::METHOD_ID,
        &canic_contracts::dto::fixture_bytes::UploadRequest::from(request.clone()),
        protocol::fixture_bytes::CHUNK_LIMITS,
        protocol::fixture_bytes::UPLOAD_RESPONSE_LIMITS,
    )?;
    if !upload_applied(
        expected,
        source_bytes,
        &status.map_err(CurrentProtocolError::Fixture)?,
    )? {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    Ok(canic_core::cdk::utils::hash::wasm_hash(&request.bytes))
}

fn verify_source_status(
    source: &RootStoreFixture,
    status: &FixtureSourceStatus,
) -> Result<(), CurrentProtocolError> {
    let content_id =
        FixtureContentApi::content_id(&source.descriptor).map_err(CurrentProtocolError::Fixture)?;
    let count = u32::try_from(source.descriptor.chunks.len())
        .map_err(|_| CurrentProtocolError::ResponseMismatch)?;
    if content_id != source.content_id
        || status.content_id != content_id
        || status.chunk_count != count
        || status.next_chunk > count
    {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    let bytes: u64 = source.descriptor.chunks[..status.next_chunk as usize]
        .iter()
        .map(|chunk| u64::from(chunk.length))
        .sum();
    if status.received_bytes != bytes || status.complete != (status.next_chunk == count) {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    Ok(())
}

fn upload_applied(
    expected: &FixtureSourceStatus,
    source_bytes: u64,
    observed: &FixtureSourceStatus,
) -> Result<bool, CurrentProtocolError> {
    let identity_matches =
        observed.content_id == expected.content_id && observed.chunk_count == expected.chunk_count;
    let cursor_valid = observed.next_chunk <= observed.chunk_count && observed.chunk_count > 0;
    let completion_valid = observed.complete == (observed.next_chunk == observed.chunk_count);
    let remaining = u64::from(observed.chunk_count.saturating_sub(observed.next_chunk));
    let bytes_valid = observed.received_bytes >= u64::from(observed.next_chunk)
        && source_bytes
            .checked_sub(observed.received_bytes)
            .is_some_and(|bytes| bytes >= remaining);
    let endpoint_valid = (!observed.complete || observed.received_bytes == source_bytes)
        && (observed.next_chunk != 0 || observed.received_bytes == 0);
    if !identity_matches || !cursor_valid || !completion_valid || !bytes_valid || !endpoint_valid {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    if observed.next_chunk == expected.next_chunk
        && observed.received_bytes != expected.received_bytes
    {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    // Store verifies every committed chunk against the content-addressed descriptor. A later
    // retained cursor proves this prefix too; accepting it prevents duplicate effects on replay.
    if observed.next_chunk > expected.next_chunk
        && observed.received_bytes <= expected.received_bytes
    {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    Ok(observed.next_chunk >= expected.next_chunk)
}
