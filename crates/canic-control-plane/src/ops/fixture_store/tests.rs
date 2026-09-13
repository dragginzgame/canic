use super::*;
use crate::storage::stable::fixture_store::FixtureStoreData;
use canic_core::{
    CANIC_WASM_CHUNK_BYTES,
    dto::fixture_provisioning::{FixtureChunkDescriptor, FixtureTargetBinding},
    ids::{
        CanisterRole, ComponentBinding, ComponentInstanceId, ReleaseBuildId, ReleaseBuildNonce,
        SubnetId,
    },
};
use sha2::{Digest, Sha256};

struct FixtureScope;

impl FixtureScope {
    fn new() -> Self {
        FixtureStore::import(FixtureStoreData::default());
        crate::storage::stable::template::WasmStoreGcStateStore::clear_for_test();
        Self
    }
}

impl Drop for FixtureScope {
    fn drop(&mut self) {
        FixtureStore::import(FixtureStoreData::default());
        crate::storage::stable::template::WasmStoreGcStateStore::clear_for_test();
    }
}

fn descriptor(chunks: &[Vec<u8>]) -> FixtureDescriptor {
    FixtureDescriptor {
        schema_version: 1,
        format_hash: [7; 32],
        encoded_length: chunks.iter().map(|chunk| chunk.len() as u64).sum(),
        chunks: chunks
            .iter()
            .map(|chunk| FixtureChunkDescriptor {
                digest: Sha256::digest(chunk).into(),
                length: u32::try_from(chunk.len()).unwrap(),
            })
            .collect(),
        completion_summary: [8; 32],
    }
}

fn authority() -> FleetSubnetWasmStoreAuthority {
    FleetSubnetWasmStoreAuthority {
        authority: crate::test_support::root_funding_request_fixture(1)
            .expected_registry
            .authority,
        placement_subnet: SubnetId::from_principal(Principal::from_slice(&[9; 29])),
        fleet_subnet_root: Principal::from_slice(&[10; 29]),
        wasm_store: Principal::from_slice(&[11; 29]),
        installation_controller: Principal::from_slice(&[12; 29]),
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
            [13; 32],
        )),
        wasm_module_hash: [14; 32],
    }
}

fn request(authority: &FleetSubnetWasmStoreAuthority, content: [u8; 32]) -> FixtureGrantRequest {
    FixtureGrantRequest {
        expected_revision: 0,
        enabled: true,
        binding: FixtureTargetBinding {
            target: ManagedCanisterBinding::Component(ComponentBinding {
                authority: authority.authority.clone(),
                component: ComponentInstanceId::from_generated_bytes([15; 32]),
                component_spec: "fixture".parse().unwrap(),
                spec_hash: [16; 32],
                role: CanisterRole::from("fixture"),
                placement_subnet: authority.placement_subnet,
                fleet_subnet_root: authority.fleet_subnet_root,
                canister_id: Principal::from_slice(&[17; 29]),
            }),
            installation: [18; 32],
            release_build_id: authority.release_build_id,
            content_id: content,
        },
    }
}

#[test]
fn fixture_store_restores_content_progress_and_exact_grant_revisions() {
    let _scope = FixtureScope::new();
    let bytes = vec![vec![1; 64], vec![2; 128]];
    let descriptor = descriptor(&bytes);
    let content = content_id(&descriptor).unwrap();
    let first = prepare(descriptor.clone(), u64::MAX, 0).unwrap();
    assert!(!first.complete);
    let authority = authority();
    let request = request(&authority, content);
    assert_eq!(
        set_grant(request.clone(), &authority, u64::MAX, 0),
        Err(FixtureStoreError::NotReady)
    );
    let upload_first = FixtureChunkUpload {
        content_id: content,
        index: 0,
        bytes: bytes[0].clone(),
    };
    assert_eq!(
        upload(
            FixtureChunkUpload {
                index: 1,
                bytes: bytes[1].clone(),
                ..upload_first
            },
            u64::MAX,
            0
        ),
        Err(FixtureStoreError::Sequence)
    );
    let first = upload(upload_first.clone(), u64::MAX, 0).unwrap();
    let saved = FixtureStore::export();
    FixtureStore::import(FixtureStoreData::default());
    FixtureStore::import(saved.clone());
    assert_eq!(upload(upload_first, 0, 0).unwrap(), first);
    assert_eq!(FixtureStore::export(), saved);
    assert_eq!(prepare(descriptor, 0, 0).unwrap(), first);
    let complete = upload(
        FixtureChunkUpload {
            content_id: content,
            index: 1,
            bytes: bytes[1].clone(),
        },
        u64::MAX,
        0,
    )
    .unwrap();
    assert!(complete.complete);
    let grant = set_grant(request.clone(), &authority, u64::MAX, 0).unwrap();
    let saved = FixtureStore::export();
    FixtureStore::import(saved.clone());
    assert_eq!(set_grant(request, &authority, 0, 0).unwrap(), grant);
    let read_request = FixtureChunkRead {
        grant: grant.clone(),
        index: 1,
    };
    assert_eq!(
        authorize_read(Principal::anonymous(), &read_request),
        Err(FixtureStoreError::Authority)
    );
    authorize_read(target_authority(&grant.binding).target, &read_request).unwrap();
    assert_eq!(read(read_request).unwrap(), bytes[1]);
    assert_eq!(FixtureStore::export(), saved);
}

#[test]
fn revoked_and_replaced_grants_fence_delayed_commands_and_reads() {
    let _scope = FixtureScope::new();
    let bytes = vec![1; 16];
    let content = prepare(descriptor(std::slice::from_ref(&bytes)), u64::MAX, 0)
        .unwrap()
        .content_id;
    upload(
        FixtureChunkUpload {
            content_id: content,
            index: 0,
            bytes,
        },
        u64::MAX,
        0,
    )
    .unwrap();
    let authority = authority();
    let original = request(&authority, content);
    let old = set_grant(original.clone(), &authority, u64::MAX, 0).unwrap();
    let revoke = FixtureGrantRequest {
        expected_revision: old.revision,
        enabled: false,
        ..original.clone()
    };
    let revoked = set_grant(revoke.clone(), &authority, u64::MAX, 0).unwrap();
    assert_eq!(set_grant(revoke, &authority, 0, 0).unwrap(), revoked);
    assert_eq!(
        set_grant(original.clone(), &authority, u64::MAX, 0),
        Err(FixtureStoreError::Conflict)
    );
    let old_read = FixtureChunkRead {
        grant: old,
        index: 0,
    };
    assert_eq!(read(old_read.clone()), Err(FixtureStoreError::Authority));
    let mut replacement = original;
    replacement.expected_revision = revoked.revision;
    replacement.binding.installation = [19; 32];
    let current = set_grant(replacement, &authority, u64::MAX, 0).unwrap();
    assert_eq!(read(old_read), Err(FixtureStoreError::Authority));
    assert_eq!(
        read(FixtureChunkRead {
            grant: current,
            index: 0
        })
        .unwrap(),
        vec![1; 16]
    );
}

#[test]
fn descriptor_and_authority_conflicts_leave_storage_unchanged() {
    let _scope = FixtureScope::new();
    let mut description = descriptor(&[vec![1; 16]]);
    description.schema_version = 2;
    assert_eq!(
        prepare(description.clone(), u64::MAX, 0),
        Err(FixtureStoreError::Bounds)
    );
    description.schema_version = 1;
    description.encoded_length += 1;
    assert_eq!(
        prepare(description.clone(), u64::MAX, 0),
        Err(FixtureStoreError::Content)
    );
    description.encoded_length -= 1;
    let content = prepare(description, u64::MAX, 0).unwrap().content_id;
    let before = FixtureStore::export();
    assert_eq!(
        upload(
            FixtureChunkUpload {
                content_id: content,
                index: 0,
                bytes: vec![2; 16]
            },
            u64::MAX,
            0
        ),
        Err(FixtureStoreError::Content)
    );
    let authority = authority();
    let mut wrong = request(&authority, content);
    let ManagedCanisterBinding::Component(component) = &mut wrong.binding.target else {
        panic!("fixture Component");
    };
    component.placement_subnet = SubnetId::from_principal(Principal::from_slice(&[20; 29]));
    assert_eq!(
        set_grant(wrong, &authority, u64::MAX, 0),
        Err(FixtureStoreError::Authority)
    );
    assert_eq!(FixtureStore::export(), before);
}

#[test]
fn fixture_capacity_includes_metadata_payload_grants_and_template_bytes() {
    let _scope = FixtureScope::new();
    let bytes = vec![5; CANIC_WASM_CHUNK_BYTES];
    let description = descriptor(std::slice::from_ref(&bytes));
    let content = prepare(description, u64::MAX, 0).unwrap().content_id;
    let before = FixtureStore::export();
    let upload_request = FixtureChunkUpload {
        content_id: content,
        index: 0,
        bytes,
    };
    upload(upload_request.clone(), u64::MAX, 0).unwrap();
    let exact = FixtureStore::occupied_bytes();
    FixtureStore::import(before.clone());
    assert_eq!(
        upload(upload_request.clone(), exact, 1),
        Err(FixtureStoreError::Capacity)
    );
    assert_eq!(FixtureStore::export(), before);
    upload(upload_request, exact, 0).unwrap();
    let before = FixtureStore::export();
    let template_before = template_bytes();
    let error =
        crate::ops::storage::template::TemplateChunkedOps::prepare_chunk_set_in_store_from_input(
            crate::dto::template::TemplateChunkSetPrepareInput {
                template_id: crate::ids::TemplateId::new("embedded:fixture-capacity"),
                version: crate::ids::TemplateVersion::new("0.110.14"),
                payload_hash: Sha256::digest([1]).to_vec(),
                payload_size_bytes: 1,
                chunk_hashes: vec![Sha256::digest([1]).to_vec()],
            },
            1,
            crate::ops::storage::template::WasmStoreLimits {
                max_store_bytes: exact,
                max_templates: None,
                max_template_versions_per_template: None,
            },
        )
        .unwrap_err();
    assert_eq!(
        error.public_error().code(),
        canic_core::diagnostics::codes::CAPACITY_LIMIT.raw_code()
    );
    assert_eq!(template_bytes(), template_before);
    assert_eq!(FixtureStore::export(), before);
    let authority = authority();
    assert_eq!(
        set_grant(request(&authority, content), &authority, exact, 0),
        Err(FixtureStoreError::Capacity)
    );
    assert_eq!(FixtureStore::export(), before);
}

#[test]
fn child_grant_preserves_component_identity_and_authenticates_only_the_child() {
    let _scope = FixtureScope::new();
    let bytes = vec![1; 16];
    let content = prepare(descriptor(std::slice::from_ref(&bytes)), u64::MAX, 0)
        .unwrap()
        .content_id;
    upload(
        FixtureChunkUpload {
            content_id: content,
            index: 0,
            bytes,
        },
        u64::MAX,
        0,
    )
    .unwrap();
    let authority = authority();
    let mut request = request(&authority, content);
    let ManagedCanisterBinding::Component(component) = request.binding.target else {
        panic!("Component fixture");
    };
    let parent = component.canister_id;
    let child = Principal::from_slice(&[22; 29]);
    request.binding.target =
        ManagedCanisterBinding::ComponentChild(canic_core::ids::ComponentChildBinding {
            component,
            parent_canister_id: parent,
            role: CanisterRole::from("shard"),
            canister_id: child,
        });
    let grant = set_grant(request.clone(), &authority, u64::MAX, 0).unwrap();
    let read = FixtureChunkRead { grant, index: 0 };
    assert_eq!(
        authorize_read(parent, &read),
        Err(FixtureStoreError::Authority)
    );
    authorize_read(child, &read).unwrap();
    request.expected_revision = 1;
    let ManagedCanisterBinding::ComponentChild(binding) = &mut request.binding.target else {
        panic!("child fixture");
    };
    binding.parent_canister_id = child;
    let before = FixtureStore::export();
    assert_eq!(
        set_grant(request, &authority, u64::MAX, 0),
        Err(FixtureStoreError::Authority)
    );
    assert_eq!(FixtureStore::export(), before);
}

#[test]
fn descriptor_admission_accounts_for_the_full_command_envelope() {
    let _scope = FixtureScope::new();
    let mut description = descriptor(&[vec![1]]);
    while candid::encode_one(StoreCommand::PrepareFixture(description.clone()))
        .unwrap()
        .len()
        <= DEFAULT_UPDATE_INGRESS_MAX_BYTES
    {
        description.chunks.push(description.chunks[0].clone());
        description.encoded_length += 1;
    }
    // The descriptor alone still fits; the actual endpoint argument does not.
    assert!(candid::encode_one(&description).unwrap().len() <= DEFAULT_UPDATE_INGRESS_MAX_BYTES);
    assert_eq!(
        prepare(description, u64::MAX, 0),
        Err(FixtureStoreError::Bounds)
    );
    assert_eq!(FixtureStore::export(), FixtureStoreData::default());
}

#[test]
fn retired_fixture_collection_is_bounded_accounted_and_restart_safe() {
    use crate::{
        ids::WasmStoreGcMode, ops::storage::template::WasmStoreGcOps,
        storage::stable::template::WasmStoreGcStateStore,
    };
    let _scope = FixtureScope::new();
    let bytes = vec![vec![1; CANIC_WASM_CHUNK_BYTES], vec![2; 64]];
    let content = prepare(descriptor(&bytes), u64::MAX, 0).unwrap().content_id;
    for (index, chunk) in bytes.into_iter().enumerate() {
        upload(
            FixtureChunkUpload {
                content_id: content,
                index: u32::try_from(index).unwrap(),
                bytes: chunk,
            },
            u64::MAX,
            0,
        )
        .unwrap();
    }
    let authority = authority();
    let grant = set_grant(request(&authority, content), &authority, u64::MAX, 0).unwrap();
    let read_request = FixtureChunkRead { grant, index: 0 };
    read(read_request.clone()).unwrap();
    let retained = FixtureStore::export();
    assert!(clear_retired_step().is_err());
    WasmStoreGcOps::prepare([21; 32], 10).unwrap();
    WasmStoreGcOps::prepare([21; 32], 11).unwrap();
    assert_eq!(FixtureStore::export(), retained);
    assert_eq!(
        read(read_request.clone()),
        Err(FixtureStoreError::Authority)
    );
    assert!(clear_retired_step().is_err());
    WasmStoreGcOps::begin(12).unwrap();
    WasmStoreGcOps::begin_clearing(13).unwrap();
    loop {
        let before = FixtureStore::export();
        let done = clear_retired_step().unwrap();
        let after = FixtureStore::export();
        // One data entry per pass; the final pass also drops the accounting key.
        assert!(before.entries.len() - after.entries.len() <= 2);
        let bytes: u64 = after
            .entries
            .iter()
            .map(|entry| (entry.key.len() + entry.value.len()) as u64)
            .sum();
        assert_eq!(FixtureStore::occupied_bytes(), bytes);
        let gc = WasmStoreGcStateStore::export();
        FixtureStore::import(after);
        WasmStoreGcStateStore::import(gc);
        assert_eq!(WasmStoreGcOps::status().mode, WasmStoreGcMode::Clearing);
        assert_eq!(
            read(read_request.clone()),
            Err(FixtureStoreError::Authority)
        );
        if done {
            break;
        }
    }
    WasmStoreGcOps::complete(14).unwrap();
    let terminal = WasmStoreGcOps::status();
    WasmStoreGcOps::prepare([21; 32], 15).unwrap();
    assert!(!WasmStoreGcOps::require_collection([21; 32]).unwrap());
    assert_eq!(WasmStoreGcOps::status(), terminal);
    assert_eq!(FixtureStore::occupied_bytes(), 0);
    assert!(FixtureStore::export().entries.is_empty());
}

#[test]
fn fixture_inventory_distinguishes_declared_uploaded_and_retiring_chunks() {
    let _scope = FixtureScope::new();
    let bytes = vec![vec![1; 64], vec![2; 128]];
    let descriptor = descriptor(&bytes);
    let content = content_id(&descriptor).unwrap();
    prepare(descriptor, u64::MAX, 0).unwrap();
    let declared = FixtureStore::inventory();
    assert_eq!(
        (
            declared.sources,
            declared.expected_chunks,
            declared.stored_chunks
        ),
        (1, 2, 0)
    );
    let entries = vec![FixtureStore::chunk_entry(content, 0, bytes[0].clone())];
    let occupied = FixtureStore::projected_bytes(&entries).unwrap();
    FixtureStore::commit(entries, occupied);
    let partial = FixtureStore::inventory();
    assert_eq!((partial.expected_chunks, partial.stored_chunks), (2, 1));
    // Retirement removes source metadata before chunks; neither count implies integrity.
    assert!(!FixtureStore::clear_retired_step());
    let retiring = FixtureStore::inventory();
    assert_eq!(
        (
            retiring.sources,
            retiring.expected_chunks,
            retiring.stored_chunks
        ),
        (0, 0, 1)
    );
    while !FixtureStore::clear_retired_step() {}
    let empty = FixtureStore::inventory();
    assert_eq!(
        (empty.sources, empty.expected_chunks, empty.stored_chunks),
        (0, 0, 0)
    );
}
