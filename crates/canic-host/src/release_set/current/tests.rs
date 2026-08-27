use super::*;

#[test]
fn current_manifest_canonical_shape_binds_both_child_digests() {
    let id = canic_core::ids::ReleaseBuildId::from_nonce(
        canic_core::ids::ReleaseBuildNonce::from_random_bytes([7; 32]),
    );
    let manifest = CurrentReleaseSetManifest {
        application_artifact_union_sha256: [8; 32],
        infrastructure_artifact_manifest_sha256: [9; 32],
        release_build_id: id,
        schema_version: CurrentReleaseSetManifest::SCHEMA_VERSION,
    };

    let bytes = manifest.canonical_bytes().expect("canonical manifest");
    let decoded: CurrentReleaseSetManifest = serde_json::from_slice(&bytes).expect("decode");

    assert_eq!(decoded, manifest);
}
