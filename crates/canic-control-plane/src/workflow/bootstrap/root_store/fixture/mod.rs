//! Module: workflow::bootstrap::root_store::fixture
//!
//! Responsibility: register release-selected sources and verify their completed Store metadata.
//! Does not own: source payload upload, target grants or application import progress.
//! Boundary: every source comes from the canonical manifest protected by installed Root authority.

use crate::{
    ops::{component_registry::ComponentRegistryOps, fixture_content},
    workflow::{
        bootstrap::root_store::{exact_adopted_store, load_and_validate_manifest},
        root_authority::validated_root_authority,
        runtime::template::WasmStoreInternalClient,
    },
};
use canic_core::{
    control_plane_support::{error::InternalError, workflow::topology::guard::TopologyGuard},
    dto::{
        fixture_provisioning::{FixtureSourceStatus, FixtureStoreError},
        fleet_subnet_root::FleetSubnetRootAuthority,
        root_store::{
            RootStoreFixture, RootStoreFixturePrepareRequest, RootStoreReleaseSetManifest,
        },
    },
    ids::CanisterRole,
};
use std::collections::BTreeSet;

/// Register one exact source while Root is Prepared. Replays return Store's retained cursor.
pub async fn prepare(
    request: RootStoreFixturePrepareRequest,
) -> Result<Result<FixtureSourceStatus, FixtureStoreError>, InternalError> {
    let _guard = TopologyGuard::try_enter()?;
    ComponentRegistryOps::require_root_store_admin_open()?;
    let (authority, _) = validated_root_authority()?;
    let _ = exact_adopted_store(authority.wasm_store_authority.wasm_store)?;
    let manifest = load_and_validate_manifest(&authority, request.bootstrap).await?;
    let Some(source) = manifest
        .fixtures
        .iter()
        .find(|source| source.role == request.role)
    else {
        return Ok(Err(FixtureStoreError::NotFound));
    };
    // Manifest reads await. Recheck the protected installation before issuing a Store effect.
    if validated_root_authority()?.0 != authority {
        return Err(InternalError::conflict());
    }
    ComponentRegistryOps::require_root_store_admin_open()?;
    WasmStoreInternalClient::new(authority.wasm_store_authority.wasm_store)
        .prepare_fixture(source.descriptor.clone())
        .await
}

/// Read the exact installed Store's source cursor without registering or uploading anything.
pub async fn source_status(
    content_id: [u8; 32],
) -> Result<Result<FixtureSourceStatus, FixtureStoreError>, InternalError> {
    let (authority, _) = validated_root_authority()?;
    WasmStoreInternalClient::new(authority.wasm_store_authority.wasm_store)
        .fixture_status(content_id)
        .await
}

pub(super) fn validate_sources<'a>(
    sources: &[RootStoreFixture],
    roles: impl Iterator<Item = &'a CanisterRole>,
) -> Result<u64, FixtureStoreError> {
    let roles = roles.collect::<BTreeSet<_>>();
    let mut previous = None;
    let mut contents = BTreeSet::new();
    let mut total = 0_u64;
    for source in sources {
        if !roles.contains(&source.role) || previous.is_some_and(|role| role >= &source.role) {
            return Err(FixtureStoreError::Authority);
        }
        previous = Some(&source.role);
        if fixture_content::content_id(&source.descriptor)? != source.content_id {
            return Err(FixtureStoreError::Content);
        }
        if contents.insert(source.content_id) {
            total = total
                .checked_add(source.descriptor.encoded_length)
                .ok_or(FixtureStoreError::Bounds)?;
        }
    }
    Ok(total)
}

pub(super) async fn require_complete(
    authority: &FleetSubnetRootAuthority,
    manifest: &RootStoreReleaseSetManifest,
) -> Result<(), InternalError> {
    let client = WasmStoreInternalClient::new(authority.wasm_store_authority.wasm_store);
    let mut contents = BTreeSet::new();
    for source in &manifest.fixtures {
        if !contents.insert(source.content_id) {
            continue;
        }
        let status = client
            .fixture_status(source.content_id)
            .await?
            .map_err(|_| InternalError::conflict())?;
        if !completed_source(source, &status) {
            return Err(InternalError::conflict());
        }
    }
    Ok(())
}

fn completed_source(source: &RootStoreFixture, status: &FixtureSourceStatus) -> bool {
    let Ok(chunk_count) = u32::try_from(source.descriptor.chunks.len()) else {
        return false;
    };
    let expected = FixtureSourceStatus {
        content_id: source.content_id,
        next_chunk: chunk_count,
        chunk_count,
        received_bytes: source.descriptor.encoded_length,
        complete: true,
    };
    status == &expected
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::dto::fixture_provisioning::{FixtureChunkDescriptor, FixtureDescriptor};

    fn source(role: &'static str) -> RootStoreFixture {
        let descriptor = FixtureDescriptor {
            schema_version: 1,
            format_hash: [1; 32],
            encoded_length: 17,
            chunks: vec![FixtureChunkDescriptor {
                digest: [2; 32],
                length: 17,
            }],
            completion_summary: [3; 32],
        };
        RootStoreFixture {
            role: CanisterRole::from(role),
            content_id: fixture_content::content_id(&descriptor).unwrap(),
            descriptor,
        }
    }

    #[test]
    fn release_sources_require_ordered_admitted_roles_and_exact_content() {
        let roles = [CanisterRole::from("alpha"), CanisterRole::from("beta")];
        let sources = [source("alpha"), source("beta")];
        assert_eq!(validate_sources(&sources, roles.iter()), Ok(17));
        assert_eq!(validate_sources(&[], roles.iter()), Ok(0));
        assert_eq!(
            validate_sources(&sources, roles[..1].iter()),
            Err(FixtureStoreError::Authority)
        );
        assert_eq!(
            validate_sources(&[source("beta"), source("alpha")], roles.iter()),
            Err(FixtureStoreError::Authority)
        );
        assert_eq!(
            validate_sources(&[source("alpha"), source("alpha")], roles.iter()),
            Err(FixtureStoreError::Authority)
        );
        let mut bad = source("alpha");
        bad.content_id = [0; 32];
        assert_eq!(
            validate_sources(&[bad], roles.iter()),
            Err(FixtureStoreError::Content)
        );
    }

    #[test]
    fn completion_checks_all_retained_source_metadata() {
        let source = source("alpha");
        let complete = FixtureSourceStatus {
            content_id: source.content_id,
            next_chunk: 1,
            chunk_count: 1,
            received_bytes: 17,
            complete: true,
        };
        assert!(completed_source(&source, &complete));
        for changed in [
            FixtureSourceStatus {
                content_id: [0; 32],
                ..complete.clone()
            },
            FixtureSourceStatus {
                next_chunk: 0,
                ..complete.clone()
            },
            FixtureSourceStatus {
                chunk_count: 2,
                ..complete.clone()
            },
            FixtureSourceStatus {
                received_bytes: 16,
                ..complete.clone()
            },
            FixtureSourceStatus {
                complete: false,
                ..complete
            },
        ] {
            assert!(!completed_source(&source, &changed));
        }
    }
}
