use super::*;
use crate::ids::{TemplateChunkingMode, TemplateManifestState, WasmStoreBinding};
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
            candid_sha256: [3; 32],
            protocol_profile_digest: ProtocolProfileDigest::from_bytes([4; 32]),
        },
    }
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

    let artifact_identities = BTreeMap::from([
        (
            first.role.clone(),
            RootArtifactIdentity {
                raw_module_hash: [3; 32],
                candid_sha256: [5; 32],
                protocol_profile_digest: ProtocolProfileDigest::from_bytes([7; 32]),
            },
        ),
        (
            second.role.clone(),
            RootArtifactIdentity {
                raw_module_hash: [4; 32],
                candid_sha256: [6; 32],
                protocol_profile_digest: ProtocolProfileDigest::from_bytes([8; 32]),
            },
        ),
    ]);
    let verified = verify_live_catalog(
        &expected,
        vec![catalog(&first), catalog(&second)],
        &artifact_identities,
    )
    .expect("exact live catalog");
    assert_eq!(verified[0].candid_sha256, [5; 32]);
    assert_eq!(
        verified[0].protocol_profile_digest,
        ProtocolProfileDigest::from_bytes([7; 32])
    );
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
            &artifact_identities,
        )
        .is_err(),
        "catalog order is part of canonical evidence"
    );
    assert!(
        verify_live_catalog(&expected, vec![catalog(&first)], &artifact_identities).is_err(),
        "a partial catalog must not verify"
    );
}

#[test]
fn post_bootstrap_catalog_projects_only_the_exact_recovery_helper_lane() {
    let release_build_id: ReleaseBuildId = "11".repeat(32).parse().expect("release-build ID");
    let application = manifest("database", 1);
    let helper = WasmStoreCatalogEntryResponse {
        role: CanisterRole::owned(POOL_LEDGER_RECOVERY_ROLE.to_string()),
        template_id: TemplateId::new(POOL_LEDGER_RECOVERY_TEMPLATE_ID),
        version: TemplateVersion::owned(release_build_id.to_string()),
        payload_hash: vec![2; 32],
        payload_size_bytes: 1_024,
    };

    assert_eq!(
        application_catalog_after_bootstrap(
            vec![catalog(&application), helper.clone()],
            release_build_id,
        )
        .expect("exact support artifact lane"),
        vec![catalog(&application)]
    );

    let mut wrong_template = helper.clone();
    wrong_template.template_id = TemplateId::new("canic:other-support-artifact");
    assert!(application_catalog_after_bootstrap(vec![wrong_template], release_build_id).is_err());
    let mut wrong_version = helper.clone();
    wrong_version.version = TemplateVersion::new("other-release");
    assert!(application_catalog_after_bootstrap(vec![wrong_version], release_build_id).is_err());
    let mut empty_payload = helper.clone();
    empty_payload.payload_hash.clear();
    empty_payload.payload_size_bytes = 0;
    assert!(application_catalog_after_bootstrap(vec![empty_payload], release_build_id).is_err());
    assert!(
        application_catalog_after_bootstrap(vec![helper.clone(), helper], release_build_id)
            .is_err(),
        "one release may expose at most one exact recovery helper"
    );
}

#[test]
fn expected_artifact_metadata_is_derived_without_local_staging_state() {
    let entry = release_set_entry("app");
    let manifest = RootStoreReleaseSetManifest {
        release_build_id: entry.artifact.release_build_id,
        component_topology_digest: ComponentTopologyDigest::from_bytes([3; 32]),
        entries: vec![entry],
    };
    let metadata = expected_artifact_manifests(&manifest, WasmStoreBinding::new("store"))
        .expect("exact artifact metadata");
    assert_eq!(metadata.len(), 1);
    assert_eq!(metadata[0].store_binding, WasmStoreBinding::new("store"));
}
