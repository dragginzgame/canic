//! Module: fleet_ensure::ops::plan_content::fixture
//!
//! Responsibility: bind retained fixture chunks to reviewed Root preparation and exact Store.
//! Does not own: network effects, descriptor selection or journal execution.
//! Boundary: hydration rejects substituted chunks, receipts, destinations and missing preparation.

use super::{
    EnsurePaths, EnsureStateError, authority, authority_error, fleet_protocol_action_kind,
    load_chunk_bytes, protocol_actions, protocol_actions_mut, request_mut, retain_object,
};
use crate::fleet_ensure::model::{CurrentFleetProtocolAction, EnsureAction, FleetEnsurePlan};
use candid::Principal;
use canic_control_plane::api::fixture_content::FixtureContentApi;
use canic_core::dto::{
    fixture_provisioning::{FixtureChunkUpload, FixtureSourceStatus},
    root_store::RootStoreFixture,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::BTreeMap;

///
/// PreparedSource
///
/// Reviewed descriptor with exact byte counts at each committed chunk boundary.
///
struct PreparedSource {
    source: RootStoreFixture,
    prefix_bytes: Vec<u64>,
}

///
/// PreparedSources
///
/// Exact Store/content bindings used to validate retained publication actions.
///
struct PreparedSources {
    entries: BTreeMap<(Principal, [u8; 32]), PreparedSource>,
}

impl PreparedSources {
    const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    fn insert(
        &mut self,
        store: Principal,
        source: RootStoreFixture,
    ) -> Result<(), EnsureStateError> {
        let content = FixtureContentApi::content_id(&source.descriptor)
            .map_err(|_| authority("fixture descriptor is invalid"))?;
        if source.content_id != content {
            return authority_error("fixture content identity differs");
        }
        let mut prefix_bytes = vec![0];
        let mut total = 0;
        for chunk in &source.descriptor.chunks {
            total += u64::from(chunk.length);
            prefix_bytes.push(total);
        }
        let key = (store, content);
        if let Some(existing) = self.entries.get(&key) {
            if existing.source != source {
                return authority_error("fixture preparation conflicts");
            }
        } else {
            self.entries.insert(
                key,
                PreparedSource {
                    source,
                    prefix_bytes,
                },
            );
        }
        Ok(())
    }

    fn verify(
        &self,
        store: &str,
        request: &FixtureChunkUpload,
        expected: &FixtureSourceStatus,
        source_bytes: u64,
    ) -> Result<(), EnsureStateError> {
        let store = Principal::from_text(store)
            .map_err(|_| authority("fixture Store identity is invalid"))?;
        let prepared = self
            .entries
            .get(&(store, request.content_id))
            .ok_or_else(|| authority("fixture publication has no exact Store preparation"))?;
        FixtureContentApi::verify_chunk(&prepared.source.descriptor, request.index, &request.bytes)
            .map_err(|_| authority("fixture bytes differ from prepared descriptor"))?;
        let next_chunk = request
            .index
            .checked_add(1)
            .ok_or_else(|| authority("fixture index overflow"))?;
        let received_bytes = *prepared
            .prefix_bytes
            .get(next_chunk as usize)
            .ok_or_else(|| authority("fixture index exceeds preparation"))?;
        let chunk_count = u32::try_from(prepared.source.descriptor.chunks.len())
            .map_err(|_| authority("fixture count overflow"))?;
        let receipt = FixtureSourceStatus {
            content_id: request.content_id,
            next_chunk,
            chunk_count,
            received_bytes,
            complete: next_chunk == chunk_count,
        };
        if expected != &receipt || source_bytes != prepared.source.descriptor.encoded_length {
            return authority_error("fixture receipt differs from prepared descriptor");
        }
        Ok(())
    }
}

pub(super) fn retain(paths: &EnsurePaths, plan: &FleetEnsurePlan) -> Result<(), EnsureStateError> {
    let mut prepared = PreparedSources::new();
    for outer in &plan.protocol_actions {
        if let EnsureAction::FleetProtocol { action, .. } = outer
            && let CurrentFleetProtocolAction::PrepareStoreFixture {
                source,
                store,
                request,
                ..
            } = action.as_ref()
        {
            if request.role != source.role {
                return authority_error("fixture preparation role differs");
            }
            prepared.insert(*store, source.clone())?;
        }
    }
    for outer in &plan.protocol_actions {
        if let EnsureAction::FleetProtocol {
            action, principal, ..
        } = outer
            && let CurrentFleetProtocolAction::PublishStoreFixtureChunk {
                request,
                expected,
                source_bytes,
                ..
            } = action.as_ref()
        {
            prepared.verify(principal, request, expected, *source_bytes)?;
            retain_object(
                paths,
                &canic_core::cdk::utils::hash::wasm_hash(&request.bytes),
                &request.bytes,
            )?;
        }
    }
    Ok(())
}

pub(super) fn hydrate(paths: &EnsurePaths, projection: &mut Value) -> Result<(), EnsureStateError> {
    let mut prepared = PreparedSources::new();
    for outer in protocol_actions(projection)? {
        if fleet_protocol_action_kind(outer)? != Some("prepare_store_fixture") {
            continue;
        }
        let action = outer
            .get("action")
            .ok_or_else(|| authority("missing fixture action"))?;
        let source: RootStoreFixture = field(action, "source")?;
        let request: canic_core::dto::root_store::RootStoreFixturePrepareRequest =
            field(action, "request")?;
        if request.role != source.role {
            return authority_error("fixture preparation role differs");
        }
        prepared.insert(field(action, "store")?, source)?;
    }
    for outer in protocol_actions_mut(projection)? {
        if fleet_protocol_action_kind(outer)? != Some("publish_store_fixture_chunk") {
            continue;
        }
        let store: String = field(outer, "principal")?;
        let action = outer
            .get("action")
            .ok_or_else(|| authority("missing fixture action"))?;
        let expected: FixtureSourceStatus = field(action, "expected")?;
        let source_bytes = field(action, "source_bytes")?;
        let projected = request_mut(outer)?;
        let (bytes, hash, size) = load_chunk_bytes(paths, projected)?;
        super::validate_chunk(&bytes, &hash, size)?;
        let request = FixtureChunkUpload {
            content_id: field_map(projected, "content_id")?,
            index: field_map(projected, "index")?,
            bytes,
        };
        prepared.verify(&store, &request, &expected, source_bytes)?;
        projected.remove("bytes_sha256");
        projected.remove("bytes_size");
        projected.insert(
            "bytes".to_string(),
            serde_json::to_value(request.bytes).map_err(|_| authority("invalid fixture bytes"))?,
        );
    }
    Ok(())
}

fn field<T: DeserializeOwned>(value: &Value, name: &str) -> Result<T, EnsureStateError> {
    serde_json::from_value(
        value
            .get(name)
            .cloned()
            .ok_or_else(|| authority("missing fixture authority"))?,
    )
    .map_err(|_| authority("invalid fixture authority"))
}

fn field_map<T: DeserializeOwned>(
    value: &serde_json::Map<String, Value>,
    name: &str,
) -> Result<T, EnsureStateError> {
    serde_json::from_value(
        value
            .get(name)
            .cloned()
            .ok_or_else(|| authority("missing fixture chunk authority"))?,
    )
    .map_err(|_| authority("invalid fixture chunk authority"))
}
