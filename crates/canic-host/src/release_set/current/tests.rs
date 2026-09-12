use super::*;
use crate::{
    release_build::plan_release_build,
    release_set::fixture::{FixtureSourceInput, compile_and_persist_fixture_artifact_manifest},
    test_support::temp_dir,
};
use canic_core::bootstrap::parse_config_model;

#[test]
fn current_manifest_canonical_shape_binds_all_child_digests() {
    let id = canic_core::ids::ReleaseBuildId::from_nonce(
        canic_core::ids::ReleaseBuildNonce::from_random_bytes([7; 32]),
    );
    let manifest = CurrentReleaseSetManifest {
        application_artifact_union_sha256: [8; 32],
        build_network: canic_core::ids::BuildNetwork::Local,
        fixture_artifact_manifest_sha256: [10; 32],
        infrastructure_artifact_manifest_sha256: [9; 32],
        release_build_id: id,
        schema_version: CurrentReleaseSetManifest::SCHEMA_VERSION,
    };

    let bytes = manifest.canonical_bytes().expect("canonical manifest");
    let decoded: CurrentReleaseSetManifest = serde_json::from_slice(&bytes).expect("decode");

    assert_eq!(decoded, manifest);
    assert_eq!(decoded.build_network, canic_core::ids::BuildNetwork::Local);
}

#[test]
fn complete_release_binds_fixture_bytes_and_rejects_substituted_receipts() {
    let root = temp_dir("complete-fixture-binding");
    let topology = fixture_topology();
    let release = plan_release_build(&root).unwrap().record.release_build_id;
    std::fs::write(root.join("rows.bin"), [7; 64]).unwrap();
    let fixtures = compile_and_persist_fixture_artifact_manifest(
        &root,
        &topology,
        release,
        &[FixtureSourceInput {
            role: "app".into(),
            format_hash: [1; 32],
            completion_summary: [2; 32],
            chunk_paths: vec!["rows.bin".into()],
        }],
    )
    .unwrap();
    // This boundary receives already-qualified application/infrastructure receipts.
    let application = PersistedApplicationArtifactUnion {
        union: crate::release_set::ApplicationArtifactUnion {
            release_build_id: release,
            fleet_component_topology_digest: topology.digest().unwrap(),
            entries: vec![],
        },
        digest: [3; 32],
        path: root.join("application-artifact-union.json"),
    };
    let infrastructure = PersistedCanicInfrastructureArtifactManifest {
        manifest: crate::release_set::CanicInfrastructureArtifactManifest {
            release_build_id: release,
            entries: vec![],
        },
        digest: [4; 32],
        path: root.join("infrastructure-artifact-manifest.json"),
    };
    let bound = compile_and_persist_current_release_set_manifest(
        &root,
        &topology,
        release,
        &application,
        &infrastructure,
        &fixtures,
    )
    .unwrap();
    assert_eq!(
        bound.manifest.fixture_artifact_manifest_sha256,
        fixtures.digest
    );
    assert_eq!(
        bound.manifest.verify_fixtures(&root, &topology).unwrap(),
        fixtures
    );
    assert!(matches!(
        bound.manifest.require_fixture_delivery(&root, &topology),
        Err(FixtureArtifactError::DeliveryUnavailable)
    ));
    let mut substituted = fixtures.clone();
    substituted.digest = [9; 32];
    assert!(matches!(
        compile_and_persist_current_release_set_manifest(
            &root,
            &topology,
            release,
            &application,
            &infrastructure,
            &substituted
        ),
        Err(CurrentReleaseSetManifestError::Fixture(
            FixtureArtifactError::Authority
        ))
    ));
    let mut substituted = bound.manifest.clone();
    substituted.fixture_artifact_manifest_sha256 = [9; 32];
    assert_ne!(
        substituted.canonical_bytes().unwrap(),
        bound.manifest.canonical_bytes().unwrap()
    );
    crate::release_build::finalize_release_build_from_manifest(&root, release, &bound.path)
        .unwrap();
    assert_eq!(
        compile_and_persist_current_release_set_manifest(
            &root,
            &topology,
            release,
            &application,
            &infrastructure,
            &fixtures
        )
        .unwrap(),
        bound
    );
    std::fs::remove_dir_all(root).unwrap();
}

fn fixture_topology() -> ComponentTopology {
    parse_config_model(
        r#"
[app]
name = "fixture"
[roles.root]
kind = "root"
[roles.app]
kind = "canister"
package = "."
[component_specs.app]
component_role = "app"
maximum_instances = 1
"#,
    )
    .unwrap()
    .compile_component_topology()
    .unwrap()
}
