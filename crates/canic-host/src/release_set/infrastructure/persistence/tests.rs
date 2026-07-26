use crate::{
    release_build::{finalize_release_build_from_manifest, plan_release_build},
    test_support::temp_dir,
};
use std::{fs, io::Write, path::Path};

use flate2::{Compression, GzBuilder};

use super::*;
use crate::release_set::WASM_MAGIC;

#[test]
fn qualified_complete_build_persists_one_exact_canonical_manifest() {
    let root = temp_dir("infrastructure-artifact-persistence");
    let plan = plan_release_build(&root).expect("plan release build");
    let release_build_id = plan.record.release_build_id;
    let outputs = build_outputs(&root, release_build_id);

    let persisted = compile_and_persist_canic_infrastructure_artifact_manifest(
        &root,
        release_build_id,
        &outputs,
    )
    .expect("persist manifest");
    let repeated = compile_and_persist_canic_infrastructure_artifact_manifest(
        &root,
        release_build_id,
        &outputs,
    )
    .expect("repeat exact persistence");

    assert_eq!(repeated, persisted);
    assert_eq!(
        persisted.path,
        root.join(".canic")
            .join("release-builds")
            .join(release_build_id.to_string())
            .join(INFRASTRUCTURE_ARTIFACT_MANIFEST_FILE)
    );
    assert_eq!(
        fs::read(&persisted.path).expect("read persisted manifest"),
        persisted
            .manifest
            .canonical_bytes()
            .expect("canonical manifest")
    );
    assert_eq!(
        load_persisted_canic_infrastructure_artifact_manifest(&root, release_build_id)
            .expect("load manifest"),
        persisted
    );
    assert_eq!(
        persisted
            .manifest
            .entries
            .iter()
            .map(|entry| entry.role)
            .collect::<Vec<_>>(),
        [
            CanicInfrastructureRole::FleetCoordinator,
            CanicInfrastructureRole::FleetSubnetRoot,
            CanicInfrastructureRole::WasmStore,
        ]
    );

    let release_set = root.join("release-set.json");
    fs::write(&release_set, b"exact release set").expect("write release set");
    finalize_release_build_from_manifest(&root, release_build_id, &release_set)
        .expect("finalize release build");
    assert_eq!(
        compile_and_persist_canic_infrastructure_artifact_manifest(
            &root,
            release_build_id,
            &outputs,
        )
        .expect("recover exact finalized manifest"),
        persisted
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn complete_build_rejects_missing_and_duplicate_roles_before_persistence() {
    let root = temp_dir("infrastructure-artifact-role-set");
    let plan = plan_release_build(&root).expect("plan release build");
    let release_build_id = plan.record.release_build_id;
    let mut outputs = build_outputs(&root, release_build_id);
    outputs.pop();

    assert!(matches!(
        compile_and_persist_canic_infrastructure_artifact_manifest(
            &root,
            release_build_id,
            &outputs,
        ),
        Err(CanicInfrastructureArtifactPersistenceError::Manifest(
            CanicInfrastructureArtifactManifestError::InfrastructureRoleSet { .. }
        ))
    ));
    assert!(!infrastructure_artifact_manifest_path(&root, release_build_id).exists());

    let mut outputs = build_outputs(&root, release_build_id);
    outputs[0].role = outputs[1].role;
    assert!(matches!(
        compile_and_persist_canic_infrastructure_artifact_manifest(
            &root,
            release_build_id,
            &outputs,
        ),
        Err(CanicInfrastructureArtifactPersistenceError::Manifest(
            CanicInfrastructureArtifactManifestError::InfrastructureRoleSet { .. }
        ))
    ));

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn complete_build_rejects_cross_build_and_out_of_root_artifacts() {
    let root = temp_dir("infrastructure-artifact-build-binding");
    let plan = plan_release_build(&root).expect("plan release build");
    let release_build_id = plan.record.release_build_id;
    let other_plan = plan_release_build(&root).expect("plan other release build");
    let mut outputs = build_outputs(&root, release_build_id);
    outputs[0].release_build_id = other_plan.record.release_build_id;

    assert!(matches!(
        compile_and_persist_canic_infrastructure_artifact_manifest(
            &root,
            release_build_id,
            &outputs,
        ),
        Err(CanicInfrastructureArtifactPersistenceError::Manifest(
            CanicInfrastructureArtifactManifestError::ReleaseBuildMismatch { .. }
        ))
    ));

    let outside = temp_dir("infrastructure-artifact-outside-root");
    fs::create_dir_all(&outside).expect("create outside root");
    let mut outputs = build_outputs(&root, release_build_id);
    let outside_wasm = outside.join("fleet_coordinator.wasm");
    fs::write(&outside_wasm, wasm(9)).expect("write outside Wasm");
    outputs[0].wasm_path = outside_wasm;
    assert!(matches!(
        compile_and_persist_canic_infrastructure_artifact_manifest(
            &root,
            release_build_id,
            &outputs,
        ),
        Err(CanicInfrastructureArtifactPersistenceError::ArtifactOutsideRoot { .. })
    ));

    fs::remove_dir_all(outside).expect("remove outside root");
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn exact_identity_rejects_conflicting_or_late_manifest_publication() {
    let root = temp_dir("infrastructure-artifact-conflict");
    let plan = plan_release_build(&root).expect("plan release build");
    let release_build_id = plan.record.release_build_id;
    let mut outputs = build_outputs(&root, release_build_id);
    compile_and_persist_canic_infrastructure_artifact_manifest(&root, release_build_id, &outputs)
        .expect("persist manifest");

    replace_output_bytes(&outputs[0], 77);
    assert!(matches!(
        compile_and_persist_canic_infrastructure_artifact_manifest(
            &root,
            release_build_id,
            &outputs,
        ),
        Err(CanicInfrastructureArtifactPersistenceError::ConflictingManifest { .. })
    ));

    let other_root = temp_dir("infrastructure-artifact-late");
    let other_plan = plan_release_build(&other_root).expect("plan other release build");
    let other_release_build_id = other_plan.record.release_build_id;
    let release_set = other_root.join("release-set.json");
    fs::write(&release_set, b"exact release set").expect("write release set");
    finalize_release_build_from_manifest(&other_root, other_release_build_id, &release_set)
        .expect("finalize other release build");
    let late_outputs = build_outputs(&other_root, other_release_build_id);
    assert!(matches!(
        compile_and_persist_canic_infrastructure_artifact_manifest(
            &other_root,
            other_release_build_id,
            &late_outputs,
        ),
        Err(CanicInfrastructureArtifactPersistenceError::FinalizedWithoutExactManifest { .. })
    ));

    outputs.clear();
    fs::remove_dir_all(other_root).expect("remove other temp root");
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn loader_rejects_noncanonical_or_identity_mismatched_manifest_bytes() {
    let root = temp_dir("infrastructure-artifact-canonical-load");
    let plan = plan_release_build(&root).expect("plan release build");
    let release_build_id = plan.record.release_build_id;
    let outputs = build_outputs(&root, release_build_id);
    let persisted = compile_and_persist_canic_infrastructure_artifact_manifest(
        &root,
        release_build_id,
        &outputs,
    )
    .expect("persist manifest");

    fs::write(
        &persisted.path,
        serde_json::to_vec_pretty(&persisted.manifest).expect("pretty manifest"),
    )
    .expect("replace with noncanonical bytes");
    assert!(matches!(
        load_persisted_canic_infrastructure_artifact_manifest(&root, release_build_id),
        Err(CanicInfrastructureArtifactPersistenceError::InvalidManifestDocument { .. })
    ));

    let other_plan = plan_release_build(&root).expect("plan other release build");
    let other_release_build_id = other_plan.record.release_build_id;
    let other_outputs = build_outputs(&root, other_release_build_id);
    let other_persisted = compile_and_persist_canic_infrastructure_artifact_manifest(
        &root,
        other_release_build_id,
        &other_outputs,
    )
    .expect("persist other manifest");
    fs::write(
        &persisted.path,
        fs::read(other_persisted.path).expect("read other manifest"),
    )
    .expect("replace with identity-mismatched manifest");
    assert!(matches!(
        load_persisted_canic_infrastructure_artifact_manifest(&root, release_build_id),
        Err(CanicInfrastructureArtifactPersistenceError::InvalidManifestDocument { .. })
    ));

    fs::remove_dir_all(root).expect("remove temp root");
}

#[cfg(unix)]
#[test]
fn complete_build_rejects_symlinked_artifact_files() {
    use std::os::unix::fs::symlink;

    let root = temp_dir("infrastructure-artifact-symlink");
    let plan = plan_release_build(&root).expect("plan release build");
    let release_build_id = plan.record.release_build_id;
    let outputs = build_outputs(&root, release_build_id);
    let linked = outputs[0].wasm_path.clone();
    let real = linked.with_file_name("real.wasm");
    fs::rename(&linked, &real).expect("move real Wasm");
    symlink(&real, &linked).expect("link Wasm");

    assert!(matches!(
        compile_and_persist_canic_infrastructure_artifact_manifest(
            &root,
            release_build_id,
            &outputs,
        ),
        Err(CanicInfrastructureArtifactPersistenceError::UnsafeArtifact { .. })
    ));

    fs::remove_dir_all(root).expect("remove temp root");
}

fn build_outputs(
    root: &Path,
    release_build_id: ReleaseBuildId,
) -> Vec<CanicInfrastructureArtifactBuildOutput> {
    [
        ("canic-wasm-store", CanicInfrastructureRole::WasmStore, 3),
        (
            "canic-control-plane",
            CanicInfrastructureRole::FleetCoordinator,
            1,
        ),
        (
            "canic-control-plane",
            CanicInfrastructureRole::FleetSubnetRoot,
            2,
        ),
    ]
    .into_iter()
    .map(|(package, role, marker)| build_output(root, release_build_id, package, role, marker))
    .collect()
}

fn build_output(
    root: &Path,
    release_build_id: ReleaseBuildId,
    package: &str,
    role: CanicInfrastructureRole,
    marker: u8,
) -> CanicInfrastructureArtifactBuildOutput {
    let artifact_root = root
        .join(".icp")
        .join("local")
        .join("canisters")
        .join(role.as_str());
    fs::create_dir_all(&artifact_root).expect("create artifact root");
    let wasm_path = artifact_root.join(format!("{}.wasm", role.as_str()));
    let wasm_gz_path = artifact_root.join(format!("{}.wasm.gz", role.as_str()));
    let bytes = wasm(marker);
    fs::write(&wasm_path, &bytes).expect("write Wasm");
    fs::write(&wasm_gz_path, gzip(&bytes)).expect("write gzip Wasm");

    CanicInfrastructureArtifactBuildOutput {
        role,
        package: package.to_string(),
        release_build_id,
        wasm_path,
        wasm_gz_path,
    }
}

fn replace_output_bytes(output: &CanicInfrastructureArtifactBuildOutput, marker: u8) {
    let bytes = wasm(marker);
    fs::write(&output.wasm_path, &bytes).expect("replace Wasm");
    fs::write(&output.wasm_gz_path, gzip(&bytes)).expect("replace gzip Wasm");
}

fn wasm(marker: u8) -> Vec<u8> {
    let mut bytes = WASM_MAGIC.to_vec();
    bytes.push(marker);
    bytes
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(bytes).expect("write gzip");
    encoder.finish().expect("finish gzip")
}
