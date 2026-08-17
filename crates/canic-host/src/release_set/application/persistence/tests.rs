//! Module: release_set::application::persistence::tests
//!
//! Responsibility: verify durable qualified application-union publication and recovery.
//! Does not own: Cargo execution, root projection, Store publication, or installation.
//! Boundary: exercises exact build, topology, artifact-file, and durable-path admission.

use std::{fs, io::Write, path::Path};

use canic_core::{
    bootstrap::{compiled::ComponentTopology, parse_config_model},
    ids::{CanisterRole, ReleaseBuildId},
};
use flate2::{Compression, GzBuilder};

use crate::{
    release_build::{finalize_release_build_from_manifest, plan_release_build},
    release_set::WASM_MAGIC,
    test_support::temp_dir,
};

use super::*;

const CONFIG: &str = r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.alpha]
kind = "canister"
package = "alpha"

[roles.shared]
kind = "canister"
package = "shared"

[component_specs.alpha]
component_role = "alpha"
maximum_instances = 2

[component_specs.alpha.children.shared]
kind = "replica"

[component_specs.alpha.spawn_grants.alpha.shared]
maximum_instances_per_parent = 2
"#;

fn topology() -> ComponentTopology {
    parse_config_model(CONFIG)
        .expect("valid Component config")
        .compile_component_topology()
        .expect("Component Topology")
}

fn target(role: &str) -> ApplicationArtifactBuildTarget {
    ApplicationArtifactBuildTarget {
        role: CanisterRole::owned(role.to_string()),
        package: format!("{role}-package"),
        wasm_relative_path: format!(".icp/local/canisters/{role}/{role}.wasm"),
        wasm_gz_relative_path: format!(".icp/local/canisters/{role}/{role}.wasm.gz"),
    }
}

fn targets() -> Vec<ApplicationArtifactBuildTarget> {
    ["shared", "alpha"].map(target).to_vec()
}

#[test]
fn qualified_build_persists_one_exact_canonical_union() {
    let root = temp_dir("application-artifact-persistence");
    let plan = plan_release_build(&root).expect("plan release build");
    let release_build_id = plan.record.release_build_id;
    let topology = topology();
    let targets = targets();
    let outputs = build_outputs(&root, release_build_id);

    let persisted = compile_and_persist_application_artifact_union(
        &root,
        &topology,
        release_build_id,
        &targets,
        &outputs,
    )
    .expect("persist application union");
    let repeated = compile_and_persist_application_artifact_union(
        &root,
        &topology,
        release_build_id,
        &targets,
        &outputs,
    )
    .expect("repeat exact persistence");

    assert_eq!(repeated, persisted);
    assert_eq!(
        persisted.path,
        root.join(".canic")
            .join("release-builds")
            .join(release_build_id.to_string())
            .join(APPLICATION_ARTIFACT_UNION_FILE)
    );
    assert_eq!(
        fs::read(&persisted.path).expect("read persisted union"),
        persisted
            .union
            .canonical_bytes(&topology)
            .expect("canonical union")
    );
    assert_eq!(
        load_persisted_application_artifact_union(&root, &topology, release_build_id)
            .expect("load application union"),
        persisted
    );
    assert!(persisted.union.entries.iter().all(|entry| {
        entry.candid_sha256 == [3; 32]
            && entry.protocol_profile_digest
                == canic_core::role_contract::ProtocolProfileDigest::from_bytes([4; 32])
    }));

    let release_set = root.join("release-set.json");
    fs::write(&release_set, b"exact release set").expect("write release set");
    finalize_release_build_from_manifest(&root, release_build_id, &release_set)
        .expect("finalize release build");
    assert_eq!(
        compile_and_persist_application_artifact_union(
            &root,
            &topology,
            release_build_id,
            &targets,
            &outputs,
        )
        .expect("recover exact finalized union"),
        persisted
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn publication_rejects_incomplete_duplicate_and_cross_build_outputs() {
    let root = temp_dir("application-artifact-output-rejection");
    let plan = plan_release_build(&root).expect("plan release build");
    let release_build_id = plan.record.release_build_id;
    let topology = topology();
    let targets = targets();
    let mut outputs = build_outputs(&root, release_build_id);
    outputs.pop();

    std::assert_matches!(
        compile_and_persist_application_artifact_union(
            &root,
            &topology,
            release_build_id,
            &targets,
            &outputs,
        ),
        Err(ApplicationArtifactUnionPersistenceError::ReleaseSet(
            ApplicationReleaseSetError::BuildOutputRoleSet { .. }
        ))
    );

    let mut outputs = build_outputs(&root, release_build_id);
    outputs[0].role = outputs[1].role.clone();
    std::assert_matches!(
        compile_and_persist_application_artifact_union(
            &root,
            &topology,
            release_build_id,
            &targets,
            &outputs,
        ),
        Err(ApplicationArtifactUnionPersistenceError::ReleaseSet(
            ApplicationReleaseSetError::BuildOutputRoleSet { .. }
        ))
    );

    let other_plan = plan_release_build(&root).expect("plan another release build");
    let mut outputs = build_outputs(&root, release_build_id);
    outputs[0].release_build_id = other_plan.record.release_build_id;
    std::assert_matches!(
        compile_and_persist_application_artifact_union(
            &root,
            &topology,
            release_build_id,
            &targets,
            &outputs,
        ),
        Err(ApplicationArtifactUnionPersistenceError::ReleaseSet(
            ApplicationReleaseSetError::ReleaseBuildMismatch { .. }
        ))
    );

    let mut outputs = build_outputs(&root, release_build_id);
    outputs[0].package = "other-package".to_string();
    std::assert_matches!(
        compile_and_persist_application_artifact_union(
            &root,
            &topology,
            release_build_id,
            &targets,
            &outputs,
        ),
        Err(ApplicationArtifactUnionPersistenceError::ReleaseSet(
            ApplicationReleaseSetError::BuildOutputPackageMismatch { .. }
        ))
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn publication_rejects_path_drift_and_artifacts_outside_the_project() {
    let root = temp_dir("application-artifact-path-rejection");
    let plan = plan_release_build(&root).expect("plan release build");
    let release_build_id = plan.record.release_build_id;
    let topology = topology();
    let mut qualified_targets = targets();
    let outputs = build_outputs(&root, release_build_id);
    qualified_targets[0].wasm_relative_path = "different/shared.wasm".to_string();

    std::assert_matches!(
        compile_and_persist_application_artifact_union(
            &root,
            &topology,
            release_build_id,
            &qualified_targets,
            &outputs,
        ),
        Err(ApplicationArtifactUnionPersistenceError::ReleaseSet(
            ApplicationReleaseSetError::BuildOutputPathMismatch { .. }
        ))
    );

    let outside = temp_dir("application-artifact-outside-project");
    fs::create_dir_all(&outside).expect("create outside root");
    let mut outputs = build_outputs(&root, release_build_id);
    let outside_wasm = outside.join("alpha.wasm");
    fs::write(&outside_wasm, wasm(9, release_build_id)).expect("write outside Wasm");
    outputs
        .iter_mut()
        .find(|output| output.role.as_str() == "alpha")
        .expect("alpha output")
        .wasm_path = outside_wasm;
    std::assert_matches!(
        compile_and_persist_application_artifact_union(
            &root,
            &topology,
            release_build_id,
            &targets(),
            &outputs,
        ),
        Err(ApplicationArtifactUnionPersistenceError::ArtifactOutsideRoot { .. })
    );

    fs::remove_dir_all(outside).expect("remove outside root");
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn publication_rejects_wasm_without_the_exact_embedded_release_identity() {
    let root = temp_dir("application-artifact-embedded-identity");
    let plan = plan_release_build(&root).expect("plan release build");
    let release_build_id = plan.record.release_build_id;
    let topology = topology();
    let outputs = build_outputs(&root, release_build_id);
    fs::write(&outputs[0].wasm_path, wasm_without_identity(9)).expect("replace raw Wasm");

    std::assert_matches!(
        compile_and_persist_application_artifact_union(
            &root,
            &topology,
            release_build_id,
            &targets(),
            &outputs,
        ),
        Err(ApplicationArtifactUnionPersistenceError::MissingReleaseBuildIdentity {
            role,
            release_build_id: observed,
            ..
        }) if role.as_str() == "shared" && observed == release_build_id
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn exact_identity_rejects_conflicting_or_late_union_publication() {
    let root = temp_dir("application-artifact-conflict");
    let plan = plan_release_build(&root).expect("plan release build");
    let release_build_id = plan.record.release_build_id;
    let topology = topology();
    let targets = targets();
    let outputs = build_outputs(&root, release_build_id);
    compile_and_persist_application_artifact_union(
        &root,
        &topology,
        release_build_id,
        &targets,
        &outputs,
    )
    .expect("persist union");

    replace_output_bytes(&outputs[0], 77);
    std::assert_matches!(
        compile_and_persist_application_artifact_union(
            &root,
            &topology,
            release_build_id,
            &targets,
            &outputs,
        ),
        Err(ApplicationArtifactUnionPersistenceError::ConflictingUnion { .. })
    );

    let other_root = temp_dir("application-artifact-late");
    let other_plan = plan_release_build(&other_root).expect("plan other release build");
    let other_release_build_id = other_plan.record.release_build_id;
    let release_set = other_root.join("release-set.json");
    fs::write(&release_set, b"exact release set").expect("write release set");
    finalize_release_build_from_manifest(&other_root, other_release_build_id, &release_set)
        .expect("finalize other release build");
    let late_outputs = build_outputs(&other_root, other_release_build_id);
    std::assert_matches!(
        compile_and_persist_application_artifact_union(
            &other_root,
            &topology,
            other_release_build_id,
            &targets,
            &late_outputs,
        ),
        Err(ApplicationArtifactUnionPersistenceError::FinalizedWithoutExactUnion { .. })
    );

    fs::remove_dir_all(other_root).expect("remove other temp root");
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn loader_rejects_noncanonical_identity_or_topology_drift() {
    let root = temp_dir("application-artifact-load-rejection");
    let plan = plan_release_build(&root).expect("plan release build");
    let release_build_id = plan.record.release_build_id;
    let topology = topology();
    let targets = targets();
    let outputs = build_outputs(&root, release_build_id);
    let persisted = compile_and_persist_application_artifact_union(
        &root,
        &topology,
        release_build_id,
        &targets,
        &outputs,
    )
    .expect("persist union");

    fs::write(
        &persisted.path,
        serde_json::to_vec_pretty(&persisted.union).expect("pretty union"),
    )
    .expect("replace with noncanonical bytes");
    std::assert_matches!(
        load_persisted_application_artifact_union(&root, &topology, release_build_id),
        Err(ApplicationArtifactUnionPersistenceError::InvalidUnionDocument { .. })
    );

    let other_plan = plan_release_build(&root).expect("plan other release build");
    let other_release_build_id = other_plan.record.release_build_id;
    let other_outputs = build_outputs(&root, other_release_build_id);
    let other_persisted = compile_and_persist_application_artifact_union(
        &root,
        &topology,
        other_release_build_id,
        &targets,
        &other_outputs,
    )
    .expect("persist other union");
    fs::write(
        &persisted.path,
        fs::read(other_persisted.path).expect("read other union"),
    )
    .expect("replace with identity-mismatched union");
    std::assert_matches!(
        load_persisted_application_artifact_union(&root, &topology, release_build_id),
        Err(ApplicationArtifactUnionPersistenceError::InvalidUnionDocument { .. })
    );

    fs::write(
        &persisted.path,
        persisted
            .union
            .canonical_bytes(&topology)
            .expect("restore canonical union"),
    )
    .expect("restore canonical union");
    let mut different_topology = topology;
    different_topology.component_specs[0].maximum_fleet_instances += 1;
    std::assert_matches!(
        load_persisted_application_artifact_union(&root, &different_topology, release_build_id,),
        Err(ApplicationArtifactUnionPersistenceError::ReleaseSet(
            ApplicationReleaseSetError::UnionTopologyDigestMismatch { .. }
        ))
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

#[cfg(unix)]
#[test]
fn publication_rejects_symlinked_artifact_files() {
    use std::os::unix::fs::symlink;

    let root = temp_dir("application-artifact-symlink");
    let plan = plan_release_build(&root).expect("plan release build");
    let release_build_id = plan.record.release_build_id;
    let topology = topology();
    let targets = targets();
    let outputs = build_outputs(&root, release_build_id);
    let linked = outputs[0].wasm_path.clone();
    let real = linked.with_file_name("real.wasm");
    fs::rename(&linked, &real).expect("move real Wasm");
    symlink(&real, &linked).expect("link Wasm");

    std::assert_matches!(
        compile_and_persist_application_artifact_union(
            &root,
            &topology,
            release_build_id,
            &targets,
            &outputs,
        ),
        Err(ApplicationArtifactUnionPersistenceError::UnsafeArtifact { .. })
    );

    fs::remove_dir_all(root).expect("remove temp root");
}

fn build_outputs(
    root: &Path,
    release_build_id: ReleaseBuildId,
) -> Vec<ApplicationArtifactFileBuildOutput> {
    [("shared", 2), ("alpha", 1)]
        .map(|(role, marker)| build_output(root, release_build_id, role, marker))
        .to_vec()
}

fn build_output(
    root: &Path,
    release_build_id: ReleaseBuildId,
    role: &str,
    marker: u8,
) -> ApplicationArtifactFileBuildOutput {
    let artifact_root = root.join(".icp").join("local").join("canisters").join(role);
    fs::create_dir_all(&artifact_root).expect("create artifact root");
    let wasm_path = artifact_root.join(format!("{role}.wasm"));
    let wasm_gz_path = artifact_root.join(format!("{role}.wasm.gz"));
    let bytes = wasm(marker, release_build_id);
    fs::write(&wasm_path, &bytes).expect("write Wasm");
    fs::write(&wasm_gz_path, gzip(&bytes)).expect("write gzip Wasm");

    ApplicationArtifactFileBuildOutput {
        role: CanisterRole::owned(role.to_string()),
        package: format!("{role}-package"),
        release_build_id,
        wasm_path,
        wasm_gz_path,
        candid_sha256: [3; 32],
        protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest::from_bytes(
            [4; 32],
        ),
    }
}

fn replace_output_bytes(output: &ApplicationArtifactFileBuildOutput, marker: u8) {
    let bytes = wasm(marker, output.release_build_id);
    fs::write(&output.wasm_path, &bytes).expect("replace Wasm");
    fs::write(&output.wasm_gz_path, gzip(&bytes)).expect("replace gzip Wasm");
}

fn wasm(marker: u8, release_build_id: ReleaseBuildId) -> Vec<u8> {
    let mut bytes = wasm_without_identity(marker);
    bytes.extend_from_slice(release_build_id.to_string().as_bytes());
    bytes
}

fn wasm_without_identity(marker: u8) -> Vec<u8> {
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
