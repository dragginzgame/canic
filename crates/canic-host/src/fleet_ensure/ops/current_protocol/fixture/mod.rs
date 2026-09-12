//! Module: fleet_ensure::ops::current_protocol::fixture
//!
//! Responsibility: compile and reconcile bounded fixture publication effects.
//! Does not own: the durable journal, target grants or application readiness.
//! Boundary: Root selects descriptors; the exact Store receives retained chunk bytes.

#[cfg(test)]
mod tests;

use super::{
    CurrentProtocolError, ResolvedProtocolAction, RootCommandFragment, RootCommandResponseFragment,
    observation, unavailable_observation,
};
use crate::{
    canister_protocol::{call_with_candid, query_with_candid},
    fleet_ensure::{model::CurrentFleetProtocolAction, ops::EffectObservation},
    icp::IcpCli,
    release_set::{
        FleetSubnetRootReleaseSetManifest,
        fixture::{FixtureArtifactManifest, read_chunk},
    },
};
use candid::Principal;
use canic_control_plane::{
    api::fixture_content::FixtureContentApi,
    dto::template::{StoreCatalogRequest, StoreCatalogResponse},
};
use canic_core::{
    dto::{
        fixture_provisioning::{FixtureChunkUpload, FixtureSourceStatus, FixtureStoreError},
        root_store::{RootStoreBootstrapRequest, RootStoreFixture, RootStoreFixturePrepareRequest},
    },
    protocol,
};
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
    let status: Result<FixtureSourceStatus, FixtureStoreError> = query_with_candid(
        icp,
        &resolved.candid_path,
        resolved.target,
        protocol::CANIC_ROOT_FIXTURE_STATUS,
        &source.content_id,
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
    let response: StoreCatalogResponse = query_with_candid(
        icp,
        &resolved.candid_path,
        resolved.target,
        protocol::CANIC_WASM_STORE_CATALOG,
        &StoreCatalogRequest::Fixture(expected.content_id),
    )?;
    let StoreCatalogResponse::Fixture(status) = response else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
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
    let response: RootCommandResponseFragment = call_with_candid(
        icp,
        &resolved.candid_path,
        resolved.target,
        protocol::CANIC_ROOT_COMMAND,
        &RootCommandFragment::PrepareStoreFixture(request.clone()),
    )?;
    let RootCommandResponseFragment::PrepareStoreFixture(status) = response else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
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
    let status: Result<FixtureSourceStatus, FixtureStoreError> = call_with_candid(
        icp,
        &resolved.candid_path,
        resolved.target,
        protocol::CANIC_WASM_STORE_PUBLISH_FIXTURE,
        request,
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
