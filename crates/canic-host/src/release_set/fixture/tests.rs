//! Module: release_set::fixture::tests
//!
//! Responsibility: qualify artifact identity, source selection and persistence recovery.
//! Does not own: canister installation or autonomous Root delivery.
//! Boundary: uses real bounded local files and exact release bindings.

use super::*;
use crate::{
    release_build::{finalize_release_build_from_manifest, plan_release_build},
    test_support::temp_dir,
};
use canic_core::bootstrap::parse_config_model;
use std::{fs, path::Path};

const CONFIG: &str = r#"
[app]
name = "fixture"
[roles.root]
kind = "root"
[roles.parent]
kind = "canister"
package = "parent"
[roles.child]
kind = "canister"
package = "child"
[component_specs.parent]
component_role = "parent"
maximum_instances = 2
[component_specs.parent.children.child]
kind = "replica"
[component_specs.parent.spawn_grants.parent.child]
maximum_instances_per_parent = 2
"#;

fn topology() -> ComponentTopology {
    parse_config_model(CONFIG)
        .unwrap()
        .compile_component_topology()
        .unwrap()
}

fn source(root: &Path) -> FixtureSourceInput {
    fs::create_dir_all(root.join("data")).unwrap();
    fs::write(root.join("data/parents.bin"), [7; 64]).unwrap();
    fs::write(root.join("data/children.bin"), [8; 128]).unwrap();
    FixtureSourceInput {
        role: "parent".into(),
        format_hash: [9; 32],
        completion_summary: [10; 32],
        chunk_paths: vec!["data/parents.bin".into(), "data/children.bin".into()],
    }
}

fn retained_chunk(root: &Path, retained: &PersistedFixtureArtifactManifest, index: u32) -> PathBuf {
    root.join(format!(
        ".canic/release-builds/{}/fixture-content/{}/{index}.bin",
        retained.manifest.release_build_id,
        canic_core::cdk::utils::hash::hex_bytes(retained.manifest.entries[0].content_id)
    ))
}

#[test]
fn canonical_role_binding_preserves_chunk_order_and_separates_release_identity() {
    let root = temp_dir("fixture-role-binding");
    let source = source(&root);
    let mut child = source.clone();
    child.role = "child".into();
    let release = plan_release_build(&root).unwrap().record.release_build_id;
    let first = compile_and_persist_fixture_artifact_manifest(
        &root,
        &topology(),
        release,
        &[source.clone(), child.clone()],
    )
    .unwrap();
    let repeated = compile_and_persist_fixture_artifact_manifest(
        &root,
        &topology(),
        release,
        &[child, source.clone()],
    )
    .unwrap();
    assert_eq!(first, repeated);
    assert_eq!(first.manifest.entries[0].role, CanisterRole::from("child"));
    assert_eq!(
        first.manifest.entries[0].content_id,
        first.manifest.entries[1].content_id
    );
    assert_eq!(
        first.manifest.entries[0]
            .descriptor
            .chunks
            .iter()
            .map(|chunk| chunk.length)
            .collect::<Vec<_>>(),
        vec![64, 128]
    );
    let another = plan_release_build(&root).unwrap().record.release_build_id;
    let second = compile_and_persist_fixture_artifact_manifest(
        &root,
        &topology(),
        another,
        std::slice::from_ref(&source),
    )
    .unwrap();
    assert_eq!(
        first.manifest.entries[0].content_id,
        second.manifest.entries[0].content_id
    );
    assert_ne!(first.digest, second.digest);
    let mut reversed = source;
    reversed.chunk_paths.reverse();
    assert_ne!(
        compile_entry(&root, &reversed).unwrap().content_id,
        second.manifest.entries[0].content_id
    );
    verify_fixture_artifacts(&root, &topology(), release, first.digest).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn interrupted_copy_resumes_without_overwrite_and_finalization_prevents_repair() {
    let root = temp_dir("fixture-copy-retry");
    let source = source(&root);
    let release = plan_release_build(&root).unwrap().record.release_build_id;
    let retained = compile_and_persist_fixture_artifact_manifest(
        &root,
        &topology(),
        release,
        std::slice::from_ref(&source),
    )
    .unwrap();
    let first_chunk = retained_chunk(&root, &retained, 0);
    let first_modified = fs::metadata(&first_chunk).unwrap().modified().unwrap();
    // Reconstruct interruption after the first durable payload, before the manifest commit.
    fs::remove_file(&retained.path).unwrap();
    fs::remove_file(retained_chunk(&root, &retained, 1)).unwrap();
    let resumed = compile_and_persist_fixture_artifact_manifest(
        &root,
        &topology(),
        release,
        std::slice::from_ref(&source),
    )
    .unwrap();
    assert_eq!(resumed, retained);
    assert_eq!(
        fs::metadata(&first_chunk).unwrap().modified().unwrap(),
        first_modified
    );
    finalize_release_build_from_manifest(&root, release, &retained.path).unwrap();
    assert_eq!(
        compile_and_persist_fixture_artifact_manifest(
            &root,
            &topology(),
            release,
            std::slice::from_ref(&source)
        )
        .unwrap(),
        retained
    );
    fs::remove_file(&first_chunk).unwrap();
    assert!(matches!(
        compile_and_persist_fixture_artifact_manifest(&root, &topology(), release, &[source]),
        Err(FixtureArtifactError::Finalized)
    ));
    assert!(!first_chunk.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn changed_sources_cannot_replace_selected_content_and_tampering_is_detected() {
    let root = temp_dir("fixture-tamper");
    let source = source(&root);
    let release = plan_release_build(&root).unwrap().record.release_build_id;
    let retained = compile_and_persist_fixture_artifact_manifest(
        &root,
        &topology(),
        release,
        std::slice::from_ref(&source),
    )
    .unwrap();
    fs::write(root.join("data/parents.bin"), [1; 64]).unwrap();
    assert!(matches!(
        compile_and_persist_fixture_artifact_manifest(&root, &topology(), release, &[source]),
        Err(FixtureArtifactError::Conflict(_))
    ));
    // Mutable authored files are no longer read by publication/recovery.
    fs::remove_dir_all(root.join("data")).unwrap();
    verify_fixture_artifacts(&root, &topology(), release, retained.digest).unwrap();
    fs::write(retained_chunk(&root, &retained, 0), [1; 64]).unwrap();
    assert!(matches!(
        verify_fixture_artifacts(&root, &topology(), release, retained.digest),
        Err(FixtureArtifactError::Store(FixtureStoreError::Content))
    ));
    assert!(matches!(
        load_fixture_artifact_manifest(&root, &topology(), release, [0; 32]),
        Err(FixtureArtifactError::Authority)
    ));
    let mut other_topology = topology();
    other_topology.component_specs[0].maximum_fleet_instances += 1;
    assert!(matches!(
        load_fixture_artifact_manifest(&root, &other_topology, release, retained.digest),
        Err(FixtureArtifactError::Authority)
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn duplicate_unknown_and_infrastructure_roles_never_publish_a_manifest() {
    let root = temp_dir("fixture-role-refusal");
    let source = source(&root);
    let release = plan_release_build(&root).unwrap().record.release_build_id;
    assert!(matches!(
        compile_and_persist_fixture_artifact_manifest(
            &root,
            &topology(),
            release,
            &[source.clone(), source.clone()]
        ),
        Err(FixtureArtifactError::Role(_))
    ));
    for role in ["unknown", "root", "wasm_store"] {
        let mut invalid = source.clone();
        invalid.role = role.into();
        assert!(matches!(
            compile_and_persist_fixture_artifact_manifest(&root, &topology(), release, &[invalid]),
            Err(FixtureArtifactError::Role(_))
        ));
    }
    assert!(
        !root
            .join(format!(
                ".canic/release-builds/{release}/fixture-artifact-manifest.json"
            ))
            .exists()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn chunk_bounds_and_workspace_path_confinement_precede_persistence() {
    let root = temp_dir("fixture-source-bounds");
    let mut source = source(&root);
    for invalid in ["../outside.bin", "/tmp/outside.bin", "data/../parents.bin"] {
        source.chunk_paths = vec![invalid.into()];
        assert!(matches!(
            compile_entry(&root, &source),
            Err(FixtureArtifactError::Path(_))
        ));
    }
    source.chunk_paths = vec!["data/parents.bin".into()];
    fs::write(root.join("data/parents.bin"), []).unwrap();
    assert!(matches!(
        compile_entry(&root, &source),
        Err(FixtureArtifactError::Store(FixtureStoreError::Bounds))
    ));
    fs::write(
        root.join("data/parents.bin"),
        vec![1; CANIC_WASM_CHUNK_BYTES],
    )
    .unwrap();
    assert_eq!(
        compile_entry(&root, &source)
            .unwrap()
            .descriptor
            .encoded_length,
        CANIC_WASM_CHUNK_BYTES as u64
    );
    fs::write(
        root.join("data/parents.bin"),
        vec![1; CANIC_WASM_CHUNK_BYTES + 1],
    )
    .unwrap();
    assert!(matches!(
        compile_entry(&root, &source),
        Err(FixtureArtifactError::Store(FixtureStoreError::Bounds))
    ));
    source.chunk_paths = vec![
        "data/children.bin".into();
        canic_core::ingress::payload::DEFAULT_UPDATE_INGRESS_MAX_BYTES / 32
    ];
    assert!(matches!(
        compile_entry(&root, &source),
        Err(FixtureArtifactError::Store(FixtureStoreError::Bounds))
    ));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn source_and_retained_links_are_refused() {
    use std::os::unix::fs::symlink;
    let root = temp_dir("fixture-links");
    let mut source = source(&root);
    symlink(root.join("data/parents.bin"), root.join("linked.bin")).unwrap();
    source.chunk_paths = vec!["linked.bin".into()];
    assert!(matches!(
        compile_entry(&root, &source),
        Err(FixtureArtifactError::Path(_))
    ));
    symlink(root.join("data"), root.join("linked-dir")).unwrap();
    source.chunk_paths = vec!["linked-dir/parents.bin".into()];
    assert!(matches!(
        compile_entry(&root, &source),
        Err(FixtureArtifactError::Path(_))
    ));
    source.chunk_paths = vec!["data/parents.bin".into()];
    let release = plan_release_build(&root).unwrap().record.release_build_id;
    let retained =
        compile_and_persist_fixture_artifact_manifest(&root, &topology(), release, &[source])
            .unwrap();
    let chunk = retained_chunk(&root, &retained, 0);
    fs::remove_file(&chunk).unwrap();
    symlink(root.join("data/parents.bin"), chunk).unwrap();
    assert!(matches!(
        verify_fixture_artifacts(&root, &topology(), release, retained.digest),
        Err(FixtureArtifactError::Path(_))
    ));
    fs::remove_dir_all(root).unwrap();
}

fn configured_workspace(root: &Path) -> PathBuf {
    fs::create_dir_all(root).unwrap();
    fs::write(root.join("canic.toml"), CONFIG).unwrap();
    for role in ["parent", "child"] {
        fs::create_dir_all(root.join(role)).unwrap();
        fs::write(
            root.join(role).join("Cargo.toml"),
            format!("[package]\nname = \"{role}\"\nversion = \"0.1.0\"\n"),
        )
        .unwrap();
    }
    fs::create_dir_all(root.join("parent/.canic/fixture")).unwrap();
    fs::write(root.join("parent/Cargo.toml"), "[package]\nname = \"parent\"\nversion = \"0.1.0\"\n[package.metadata.canic]\nfixture = \".canic/fixture/source.json\"\n").unwrap();
    fs::write(root.join("parent/.canic/fixture/rows.bin"), [3; 64]).unwrap();
    fs::write(root.join("parent/.canic/fixture/source.json"), serde_json::to_vec(&serde_json::json!({
        "format_hash": "01".repeat(32), "completion_summary": "02".repeat(32), "chunk_paths": ["rows.bin"]
    })).unwrap()).unwrap();
    root.join("canic.toml")
}

#[test]
fn configured_sources_bind_metadata_and_payload_even_in_excluded_directories() {
    let root = temp_dir("fixture-configured");
    let config = configured_workspace(&root);
    let selected = load_configured_fixture_sources(&root, &config).unwrap();
    assert_eq!(selected.inputs.len(), 1);
    assert_eq!(selected.inputs[0].role, CanisterRole::from("parent"));
    let rows = root.join("parent/.canic/fixture/rows.bin");
    assert!(selected.source_files.contains_key(&rows));
    assert!(
        selected
            .source_files
            .contains_key(&root.join("child/Cargo.toml"))
    );
    selected.verify_unchanged(&root, &config).unwrap();
    fs::write(&rows, [4; 64]).unwrap();
    assert!(matches!(
        selected.verify_unchanged(&root, &config),
        Err(FixtureArtifactError::ChangedInputs)
    ));
    fs::write(&rows, [3; 64]).unwrap();
    fs::write(
        root.join("child/Cargo.toml"),
        "[package]\nname = \"changed\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    assert!(matches!(
        selected.verify_unchanged(&root, &config),
        Err(FixtureArtifactError::ChangedInputs)
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn configured_source_manifest_is_strict_and_cannot_escape_its_directory() {
    let root = temp_dir("fixture-config-refusal");
    let config = configured_workspace(&root);
    let write = |document: serde_json::Value| {
        fs::write(
            root.join("parent/.canic/fixture/source.json"),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();
    };
    write(
        serde_json::json!({"format_hash": "01".repeat(32), "completion_summary": "02".repeat(32), "chunk_paths": ["../rows.bin"]}),
    );
    assert!(matches!(
        load_configured_fixture_sources(&root, &config),
        Err(FixtureArtifactError::Path(_))
    ));
    write(
        serde_json::json!({"format_hash": "invalid", "completion_summary": "02".repeat(32), "chunk_paths": ["rows.bin"]}),
    );
    assert!(matches!(
        load_configured_fixture_sources(&root, &config),
        Err(FixtureArtifactError::Content)
    ));
    write(
        serde_json::json!({"format_hash": "01".repeat(32), "completion_summary": "02".repeat(32), "chunk_paths": ["rows.bin"], "unrecognised": true}),
    );
    assert!(matches!(
        load_configured_fixture_sources(&root, &config),
        Err(FixtureArtifactError::Json(_))
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn restored_source_does_not_hide_different_bytes_retained_during_build() {
    let root = temp_dir("fixture-transient-source");
    let config = configured_workspace(&root);
    let selected = load_configured_fixture_sources(&root, &config).unwrap();
    let release = plan_release_build(&root).unwrap().record.release_build_id;
    let rows = root.join("parent/.canic/fixture/rows.bin");
    fs::write(&rows, [4; 64]).unwrap();
    let retained = compile_and_persist_fixture_artifact_manifest(
        &root,
        &topology(),
        release,
        &selected.inputs,
    )
    .unwrap();
    fs::write(rows, [3; 64]).unwrap();
    selected.verify_unchanged(&root, &config).unwrap();
    assert!(matches!(
        selected.verify_retained(&retained),
        Err(FixtureArtifactError::ChangedInputs)
    ));
    fs::remove_dir_all(root).unwrap();
}
