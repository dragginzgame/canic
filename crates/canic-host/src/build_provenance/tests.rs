use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    canister_build::{
        ArtifactTransformKind, ArtifactTransformMode, ArtifactTransformOutcome,
        ArtifactTransformOutput, CanisterArtifactBuildOutput, CanisterBuildProfile,
    },
    evidence_envelope::{CommandProvenanceV1, EvidenceTargetKindV1, PayloadSchemaRefV1},
    test_support::temp_dir,
};

#[test]
fn build_provenance_schema_is_stable() {
    assert_eq!(
        build_provenance_schema(),
        PayloadSchemaRefV1::stable("canic.build_provenance.v1", "1")
    );
}

#[test]
fn unknown_source_provenance_is_explicit() {
    let root = temp_dir("canic-build-provenance-no-git");
    fs::create_dir_all(&root).expect("create root");

    let provenance = source_provenance(&root);

    fs::remove_dir_all(&root).expect("remove root");
    assert_eq!(provenance.vcs, SourceVcsV1::Unknown);
    assert_eq!(provenance.dirty_policy, SourceDirtyPolicyV1::Unknown);
}

#[test]
fn source_provenance_requires_selected_git_worktree_root() {
    let temp = temp_dir("canic-build-provenance-parent-git");
    let root = canic_repo_root()
        .join("target")
        .join(temp.file_name().expect("temp path has file name"));
    fs::create_dir_all(&root).expect("create root");

    let provenance = source_provenance(&root);

    fs::remove_dir_all(&root).expect("remove root");
    assert_eq!(provenance.vcs, SourceVcsV1::Unknown);
    assert_eq!(provenance.dirty_policy, SourceDirtyPolicyV1::Unknown);
}

#[test]
fn artifact_provenance_records_wasm_and_gzip_separately() {
    let root = temp_dir("canic-build-provenance-artifacts");
    let artifact_root = root.join(".icp/local/canisters/app");
    fs::create_dir_all(&artifact_root).expect("create artifacts");
    let wasm_path = artifact_root.join("app.wasm");
    let wasm_gz_path = artifact_root.join("app.wasm.gz");
    let did_path = artifact_root.join("app.did");
    fs::write(&wasm_path, b"wasm").expect("write wasm");
    fs::write(&wasm_gz_path, b"gzip").expect("write gzip");

    let request = sample_request(
        &root,
        CanisterArtifactBuildOutput {
            artifact_root,
            wasm_path,
            wasm_gz_path,
            did_path,
            transforms: Vec::new(),
        },
    );
    let artifacts = artifact_provenance(&request).expect("artifact provenance");

    fs::remove_dir_all(&root).expect("remove root");
    assert_eq!(artifacts.len(), 2);
    assert_eq!(artifacts[0].artifact_kind, ArtifactProvenanceKindV1::Wasm);
    assert_eq!(
        artifacts[1].artifact_kind,
        ArtifactProvenanceKindV1::WasmGzip
    );
    assert_ne!(artifacts[0].sha256, artifacts[1].sha256);
}

#[test]
fn build_provenance_envelope_wraps_stable_payload() {
    let root = temp_dir("canic-build-provenance-envelope");
    write_sample_workspace(&root, "demo", "app");
    let output = write_sample_artifacts(&root, "app");
    let request = BuildProvenanceRequest {
        fleet: "demo".to_string(),
        role: "app".to_string(),
        network: "staging".to_string(),
        build_network: "ic".to_string(),
        profile: CanisterBuildProfile::Fast,
        workspace_root: root.clone(),
        config_path: root.join("fleets/demo/canic.toml"),
        output,
        command: sample_command(),
        generated_at: "unix:1".to_string(),
        canic_version: "0.0.0-test".to_string(),
    };

    let envelope = build_provenance_envelope(&request).expect("build envelope");
    let payload = serde_json::from_value::<BuildProvenanceV1>(envelope.payload.clone())
        .expect("decode payload");

    fs::remove_dir_all(&root).expect("remove root");
    assert_eq!(envelope.target.kind, EvidenceTargetKindV1::Artifact);
    assert_eq!(envelope.target.fleet.as_deref(), Some("demo"));
    assert_eq!(envelope.target.role.as_deref(), Some("app"));
    assert_eq!(envelope.target.network.as_deref(), Some("staging"));
    assert!(envelope.inputs.iter().any(|input| {
        input.kind == "build_environment"
            && input.note.as_deref() == Some("environment=staging;build_network=ic")
    }));
    assert_eq!(envelope.payload_schema, build_provenance_schema());
    assert_eq!(payload.cargo.package_metadata_fleet, "demo");
    assert_eq!(payload.cargo.package_metadata_role, "app");
    assert!(payload.cargo.cargo_lock_sha256.is_some());
    assert_eq!(payload.artifacts.len(), 2);
    assert_eq!(payload.transforms.len(), 2);
    assert_eq!(payload.transforms[0].role, "app");
    assert_eq!(
        payload.transforms[0].transform,
        ArtifactTransformKindV1::Shrink
    );
    assert_eq!(
        payload.transforms[0].outcome,
        ArtifactTransformOutcomeV1::Applied
    );
    assert_eq!(
        payload.transforms[0].tool_version.as_deref(),
        Some("ic-wasm 0.test")
    );
    assert_eq!(
        payload.transforms[1].outcome,
        ArtifactTransformOutcomeV1::NotRequested
    );
    assert_eq!(payload.transforms[1].tool_version, None);
}

#[test]
fn build_provenance_rejects_transform_outcome_without_matching_tool_version() {
    let root = temp_dir("canic-build-provenance-transform-version");
    write_sample_workspace(&root, "demo", "app");
    let mut output = write_sample_artifacts(&root, "app");
    output.transforms[0].tool_version = None;
    let request = sample_request(&root, output);

    build_provenance_envelope(&request)
        .expect_err("applied transform without tool version must reject");

    fs::remove_dir_all(&root).expect("remove root");
}

fn sample_request(root: &Path, output: CanisterArtifactBuildOutput) -> BuildProvenanceRequest {
    BuildProvenanceRequest {
        fleet: "demo".to_string(),
        role: "app".to_string(),
        network: "local".to_string(),
        build_network: "local".to_string(),
        profile: CanisterBuildProfile::Fast,
        workspace_root: root.to_path_buf(),
        config_path: root.join("fleets/demo/canic.toml"),
        output,
        command: sample_command(),
        generated_at: "unix:1".to_string(),
        canic_version: "0.0.0-test".to_string(),
    }
}

fn sample_command() -> CommandProvenanceV1 {
    CommandProvenanceV1 {
        name: "canic build".to_string(),
        argv_normalized: vec!["canic".to_string(), "build".to_string()],
        argv_redactions: Vec::new(),
        format: "provenance".to_string(),
    }
}

fn write_sample_workspace(root: &Path, fleet: &str, role: &str) {
    let package_dir = root.join("fleets").join(fleet).join(role);
    fs::create_dir_all(package_dir.join("src")).expect("create package");
    fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[workspace]
members = ["fleets/{fleet}/{role}"]
resolver = "3"
"#
        ),
    )
    .expect("write workspace manifest");
    fs::write(root.join("Cargo.lock"), "# lock\n").expect("write lock");
    fs::write(
        root.join("fleets").join(fleet).join("canic.toml"),
        format!(
            r#"[fleet]
name = "{fleet}"

[roles.root]
kind = "root"
package = "root"

[roles.{role}]
kind = "canister"
package = "{role}"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.{role}]
kind = "service"
"#
        ),
    )
    .expect("write canic config");
    fs::write(
        package_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "canister_{fleet}_{role}"
version = "0.0.0"
edition = "2024"

[package.metadata.canic]
fleet = "{fleet}"
role = "{role}"
"#
        ),
    )
    .expect("write package manifest");
    fs::write(package_dir.join("src/lib.rs"), "").expect("write lib");
}

fn write_sample_artifacts(root: &Path, role: &str) -> CanisterArtifactBuildOutput {
    let artifact_root = root.join(".icp/local/canisters").join(role);
    fs::create_dir_all(&artifact_root).expect("create artifacts");
    let wasm_path = artifact_root.join(format!("{role}.wasm"));
    let wasm_gz_path = artifact_root.join(format!("{role}.wasm.gz"));
    let did_path = artifact_root.join(format!("{role}.did"));
    fs::write(&wasm_path, b"wasm").expect("write wasm");
    fs::write(&wasm_gz_path, b"gzip").expect("write gzip");

    CanisterArtifactBuildOutput {
        artifact_root,
        wasm_path,
        wasm_gz_path,
        did_path,
        transforms: vec![
            ArtifactTransformOutput {
                role: role.to_string(),
                transform: ArtifactTransformKind::Shrink,
                mode: ArtifactTransformMode::Optional,
                tool: "ic-wasm".to_string(),
                tool_version: Some("ic-wasm 0.test".to_string()),
                outcome: ArtifactTransformOutcome::Applied,
            },
            ArtifactTransformOutput::not_requested(role, ArtifactTransformKind::CandidMetadata),
        ],
    }
}

fn canic_repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|path| path.join(".git").exists())
        .expect("Canic repository root has .git")
        .to_path_buf()
}
