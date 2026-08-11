use super::*;
use crate::{
    dto::template::TemplateManifestInput,
    ids::{TemplateChunkingMode, TemplateManifestState, WasmStoreBinding},
    storage::stable::template::{
        TemplateChunkSetStateStore, TemplateChunkStore, TemplateManifestStateStore,
    },
};
use canic_core::{dto::root_store::RootStoreArtifact, ids::ComponentTopologyDigest};

fn manifest(role: &str, byte: u8) -> TemplateManifestResponse {
    TemplateManifestResponse {
        template_id: TemplateId::owned(format!("{ROOT_STORE_ARTIFACT_TEMPLATE_PREFIX}{role}")),
        role: CanisterRole::owned(role.to_string()),
        version: TemplateVersion::new("release-build"),
        payload_hash: vec![byte; 32],
        payload_size_bytes: 1_024,
        store_binding: WasmStoreBinding::new("bootstrap"),
        chunking_mode: TemplateChunkingMode::Chunked,
        manifest_state: TemplateManifestState::Approved,
        approved_at: Some(0),
        created_at: 0,
    }
}

fn catalog(manifest: &TemplateManifestResponse) -> WasmStoreCatalogEntryResponse {
    WasmStoreCatalogEntryResponse {
        role: manifest.role.clone(),
        template_id: manifest.template_id.clone(),
        version: manifest.version.clone(),
        payload_hash: manifest.payload_hash.clone(),
        payload_size_bytes: manifest.payload_size_bytes,
    }
}

fn release_set_entry(package: &str) -> RootStoreReleaseSetEntry {
    RootStoreReleaseSetEntry {
        component_spec: "app".parse().expect("Component Spec ID"),
        kind: RootStoreReleaseSetEntryKind::Component,
        artifact: RootStoreArtifact {
            role: CanisterRole::from("app"),
            package: package.to_string(),
            release_build_id: "00".repeat(32).parse().expect("release-build ID"),
            wasm_relative_path: ".icp/local/canisters/app/app.wasm".to_string(),
            wasm_size_bytes: 1,
            wasm_sha256_hex: "01".repeat(32),
            wasm_gz_relative_path: ".icp/local/canisters/app/app.wasm.gz".to_string(),
            wasm_gz_size_bytes: 1,
            wasm_gz_sha256_hex: "02".repeat(32),
        },
    }
}

fn reset_staged_templates() {
    TemplateManifestStateStore::clear_for_test();
    TemplateChunkSetStateStore::clear_for_test();
    TemplateChunkStore::clear_for_test();
}

#[test]
fn sha256_decoder_accepts_only_canonical_lowercase_hex() {
    assert_eq!(
        decode_sha256(&"0f".repeat(32)).expect("canonical digest"),
        [15; 32]
    );
    assert!(decode_sha256(&"0F".repeat(32)).is_err());
    assert!(decode_sha256("0f").is_err());
}

#[test]
fn runtime_projection_does_not_conflate_package_selector_with_cargo_identity() {
    let configured_selector = release_set_entry("app");
    let canonical_cargo_package = release_set_entry("demo_fleet_app");

    assert_eq!(
        RootReleaseSetEntryAuthority::from_entry(&configured_selector),
        RootReleaseSetEntryAuthority::from_entry(&canonical_cargo_package)
    );
}

#[test]
fn live_catalog_must_equal_the_complete_ordered_release_set() {
    let first = manifest("database_a", 1);
    let second = manifest("database_b", 2);
    let expected = vec![first.clone(), second.clone()];

    let module_hashes = BTreeMap::from([
        (first.role.clone(), [3; 32]),
        (second.role.clone(), [4; 32]),
    ]);
    let verified = verify_live_catalog(
        &expected,
        vec![catalog(&first), catalog(&second)],
        &module_hashes,
    )
    .expect("exact live catalog");
    assert_eq!(
        verified
            .into_iter()
            .map(|entry| entry.role)
            .collect::<Vec<_>>(),
        vec![
            CanisterRole::from("database_a"),
            CanisterRole::from("database_b")
        ]
    );

    assert!(
        verify_live_catalog(
            &expected,
            vec![catalog(&second), catalog(&first)],
            &module_hashes,
        )
        .is_err(),
        "catalog order is part of canonical evidence"
    );
    assert!(
        verify_live_catalog(&expected, vec![catalog(&first)], &module_hashes).is_err(),
        "a partial catalog must not verify"
    );
}

#[test]
fn metadata_status_path_does_not_rehash_staged_payload_bytes() {
    reset_staged_templates();
    let entry = release_set_entry("app");
    let manifest = RootStoreReleaseSetManifest {
        release_build_id: entry.artifact.release_build_id,
        component_topology_digest: ComponentTopologyDigest::from_bytes([3; 32]),
        entries: vec![entry],
    };
    let artifact = &manifest.entries[0].artifact;
    TemplateManifestOps::replace_approved_from_input(TemplateManifestInput {
        template_id: artifact_template_id(&artifact.role),
        role: artifact.role.clone(),
        version: TemplateVersion::owned(manifest.release_build_id.to_string()),
        payload_hash: decode_sha256(&artifact.wasm_gz_sha256_hex)
            .expect("payload digest")
            .to_vec(),
        payload_size_bytes: artifact.wasm_gz_size_bytes,
        store_binding: WASM_STORE_BOOTSTRAP_BINDING,
        chunking_mode: TemplateChunkingMode::Chunked,
        manifest_state: TemplateManifestState::Approved,
        approved_at: Some(1),
        created_at: 1,
    });

    let metadata = exact_staged_manifest_metadata(&manifest)
        .expect("exact approved metadata does not require chunk reads");
    assert_eq!(metadata.len(), 1);
    exact_staged_manifests(&manifest)
        .expect_err("bootstrap validation still requires complete staged chunks");
    reset_staged_templates();
}
