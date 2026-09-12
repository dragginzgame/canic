//! Deterministic fixture content validation, storage admission and grant transitions.
//!
//! Owns conversions and synchronous writes; callers own Root authentication,
//! release publication, installed-target verification and lifecycle sequencing.

#[cfg(test)]
mod tests;

use crate::{
    dto::template::StoreCommand,
    ops::fixture_content::{content_id, verify_chunk},
    storage::stable::{
        fixture_store::{FixtureSourceRecord, FixtureStore, FixtureStoreEntryRecord},
        template::{TemplateChunkSetStateStore, TemplateChunkStore, TemplateManifestStateStore},
    },
    view::fixture_store::FixtureTargetAuthority,
};
use candid::Principal;
use canic_core::{
    dto::fixture_provisioning::{
        FixtureChunkRead, FixtureChunkUpload, FixtureDescriptor, FixtureGrant, FixtureGrantRequest,
        FixtureSourceStatus, FixtureStoreError, FixtureTargetBinding,
    },
    ids::{FleetSubnetWasmStoreAuthority, ManagedCanisterBinding},
    ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES,
};

/// Read the existing executable-template allocation counters for shared admission.
pub fn template_bytes() -> u64 {
    TemplateChunkStore::occupied_bytes()
        .saturating_add(TemplateChunkSetStateStore::occupied_bytes())
        .saturating_add(TemplateManifestStateStore::occupied_bytes())
}

/// Admit descriptor bytes once; exact retry preserves the existing source cursor.
pub fn prepare(
    descriptor: FixtureDescriptor,
    maximum_bytes: u64,
    template_bytes: u64,
) -> Result<FixtureSourceStatus, FixtureStoreError> {
    let content = content_id(&descriptor)?;
    if let Some(record) = FixtureStore::source(content) {
        if record.descriptor != descriptor {
            return Err(FixtureStoreError::Conflict);
        }
        return Ok(status(content, &record));
    }
    let record = FixtureSourceRecord {
        descriptor,
        next_chunk: 0,
        received_bytes: 0,
    };
    retain(
        vec![FixtureStore::source_entry(content, &record)],
        maximum_bytes,
        template_bytes,
    )?;
    Ok(status(content, &record))
}

/// Validate before atomically retaining the payload, progress and byte ledger.
pub fn upload(
    request: FixtureChunkUpload,
    maximum_bytes: u64,
    template_bytes: u64,
) -> Result<FixtureSourceStatus, FixtureStoreError> {
    let mut record = FixtureStore::source(request.content_id).ok_or(FixtureStoreError::NotFound)?;
    verify_chunk(&record.descriptor, request.index, &request.bytes)?;
    if request.index < record.next_chunk {
        return Ok(status(request.content_id, &record));
    }
    if request.index != record.next_chunk {
        return Err(FixtureStoreError::Sequence);
    }
    record.next_chunk += 1;
    record.received_bytes += request.bytes.len() as u64;
    retain(
        vec![
            FixtureStore::source_entry(request.content_id, &record),
            FixtureStore::chunk_entry(request.content_id, request.index, request.bytes),
        ],
        maximum_bytes,
        template_bytes,
    )?;
    Ok(status(request.content_id, &record))
}

/// Project source completion from its authoritative ingestion record.
pub fn source_status(content: [u8; 32]) -> Result<FixtureSourceStatus, FixtureStoreError> {
    FixtureStore::source(content)
        .map(|record| status(content, &record))
        .ok_or(FixtureStoreError::NotFound)
}

/// Fence stale Root intents using a retained target revision, including revocation.
pub fn set_grant(
    request: FixtureGrantRequest,
    authority: &FleetSubnetWasmStoreAuthority,
    maximum_bytes: u64,
    template_bytes: u64,
) -> Result<FixtureGrant, FixtureStoreError> {
    validate_authority(&request, authority)?;
    let target = target_authority(&request.binding).target;
    let previous = FixtureStore::grant(target);
    let revision = request
        .expected_revision
        .checked_add(1)
        .ok_or(FixtureStoreError::Bounds)?;
    let next = FixtureGrant {
        revision,
        binding: request.binding,
        enabled: request.enabled,
    };
    if previous.as_ref() == Some(&next) {
        return Ok(next);
    }
    if previous.as_ref().map_or(0, |grant| grant.revision) != request.expected_revision {
        return Err(FixtureStoreError::Conflict);
    }
    if next.enabled {
        if !source_status(next.binding.content_id)?.complete {
            return Err(FixtureStoreError::NotReady);
        }
    } else if previous
        .as_ref()
        .is_none_or(|grant| grant.binding != next.binding)
    {
        return Err(FixtureStoreError::Conflict);
    }
    retain(
        vec![FixtureStore::grant_entry(target, &next)],
        maximum_bytes,
        template_bytes,
    )?;
    Ok(next)
}

/// Require the exact granted target Principal and complete current grant binding.
pub fn authorize_read(
    caller: Principal,
    request: &FixtureChunkRead,
) -> Result<(), FixtureStoreError> {
    #[cfg(feature = "wasm-store-canister")]
    crate::ops::storage::template::WasmStoreGcOps::require_writable()
        .map_err(|_| FixtureStoreError::Authority)?;
    if caller != target_authority(&request.grant.binding).target {
        return Err(FixtureStoreError::Authority);
    }
    let current = FixtureStore::grant(caller).ok_or(FixtureStoreError::Authority)?;
    if !current.enabled || current != request.grant {
        return Err(FixtureStoreError::Authority);
    }
    Ok(())
}

/// Fetch one retained chunk after admission; revalidate the revision at access.
pub fn read(request: FixtureChunkRead) -> Result<Vec<u8>, FixtureStoreError> {
    // Recheck the exact revision at the local read boundary as well as admission.
    authorize_read(target_authority(&request.grant.binding).target, &request)?;
    FixtureStore::chunk(request.grant.binding.content_id, request.index)
        .ok_or(FixtureStoreError::NotFound)
}

/// Reclaim one fixture entry only inside the existing one-way Store clearing phase.
#[cfg(feature = "wasm-store-canister")]
pub fn clear_retired_step() -> Result<bool, canic_core::dto::error::Error> {
    if crate::ops::storage::template::WasmStoreGcOps::status().mode
        != crate::ids::WasmStoreGcMode::Clearing
    {
        return Err(canic_core::dto::error::Error::from_registered(
            canic_core::diagnostics::codes::STATE_CONFLICT,
        ));
    }
    Ok(FixtureStore::clear_retired_step())
}

/// Project the latest enabled grant or disabled revision tombstone.
pub fn grant_status(target: Principal) -> Option<FixtureGrant> {
    FixtureStore::grant(target)
}

fn validate_authority(
    request: &FixtureGrantRequest,
    authority: &FleetSubnetWasmStoreAuthority,
) -> Result<(), FixtureStoreError> {
    let binding = &request.binding;
    let projected = target_authority(binding);
    let component = projected.component;
    let same_scope = component.authority == authority.authority
        && component.placement_subnet == authority.placement_subnet
        && component.fleet_subnet_root == authority.fleet_subnet_root;
    let valid_target = [component.canister_id, projected.target, projected.parent]
        .into_iter()
        .all(|principal| {
            principal != Principal::anonymous() && principal != Principal::management_canister()
        });
    let valid_relationship = projected.target != projected.parent
        && match &binding.target {
            ManagedCanisterBinding::Component(_) => true,
            ManagedCanisterBinding::ComponentChild(child) => {
                child.canister_id != component.canister_id
            }
        };
    if !same_scope
        || !valid_target
        || !valid_relationship
        || binding.release_build_id != authority.release_build_id
        || binding.installation == [0; 32]
    {
        return Err(FixtureStoreError::Authority);
    }
    if candid::encode_one(StoreCommand::SetFixtureGrant(Box::new(request.clone())))
        .map_err(|_| FixtureStoreError::Bounds)?
        .len()
        > DEFAULT_UPDATE_INGRESS_MAX_BYTES
    {
        return Err(FixtureStoreError::Bounds);
    }
    Ok(())
}

fn status(content_id: [u8; 32], record: &FixtureSourceRecord) -> FixtureSourceStatus {
    FixtureSourceStatus {
        content_id,
        next_chunk: record.next_chunk,
        chunk_count: u32::try_from(record.descriptor.chunks.len())
            .expect("admitted fixture chunk count"),
        received_bytes: record.received_bytes,
        complete: record.next_chunk as usize == record.descriptor.chunks.len()
            && record.received_bytes == record.descriptor.encoded_length,
    }
}

const fn target_authority(binding: &FixtureTargetBinding) -> FixtureTargetAuthority<'_> {
    match &binding.target {
        ManagedCanisterBinding::Component(component) => FixtureTargetAuthority {
            component,
            target: component.canister_id,
            parent: component.fleet_subnet_root,
        },
        ManagedCanisterBinding::ComponentChild(child) => FixtureTargetAuthority {
            component: &child.component,
            target: child.canister_id,
            parent: child.parent_canister_id,
        },
    }
}

fn retain(
    entries: Vec<FixtureStoreEntryRecord>,
    maximum_bytes: u64,
    template_bytes: u64,
) -> Result<(), FixtureStoreError> {
    let projected = FixtureStore::projected_bytes(&entries).ok_or(FixtureStoreError::Capacity)?;
    let combined = projected
        .checked_add(template_bytes)
        .ok_or(FixtureStoreError::Capacity)?;
    if combined > maximum_bytes {
        return Err(FixtureStoreError::Capacity);
    }
    FixtureStore::commit(entries, projected);
    Ok(())
}
