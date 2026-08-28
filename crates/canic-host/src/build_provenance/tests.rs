use super::*;
use canic_core::ids::BuildNetwork;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    canister_build::{
        ArtifactTransformKind, ArtifactTransformOutcome, ArtifactTransformOutput,
        CanisterArtifactBuildOutput, CanisterBuildProfile, WasmArtifactMetrics,
        WasmTransformMetrics,
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
            package_name: "app-package".to_string(),
            package_version: "0.101.51".to_string(),
            protocol_release_identity: "0.101.51".to_string(),
            protocol_role: canic_core::ids::CanisterRole::new("app"),
            protocol_capabilities: std::collections::BTreeSet::new(),
            artifact_root,
            wasm_path,
            wasm_gz_path,
            did_path,
            candid_sha256: [3; 32],
            protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest::from_bytes(
                [4; 32],
            ),
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
fn release_optimization_provenance_records_binaryen_metrics() {
    let root = temp_dir("canic-build-provenance-optimization");
    write_sample_workspace(&root, "demo", "app");
    let mut output = write_sample_artifacts(&root, "app");
    let expected_tool_sha256 = crate::binaryen::current_binaryen_authority()
        .expect("supported test platform")
        .executable_sha256()
        .to_string();
    output.transforms[2] = ArtifactTransformOutput {
        transform: ArtifactTransformKind::Optimize,
        tool_version: Some("wasm-opt version 108 (version_108)".to_string()),
        tool_sha256: Some(expected_tool_sha256.clone()),
        outcome: ArtifactTransformOutcome::Applied,
        metrics: Some(WasmTransformMetrics {
            before: WasmArtifactMetrics {
                raw_bytes: 100,
                gzip_bytes: 80,
                code_section_bytes: 70,
                data_section_bytes: 20,
                defined_functions: 10,
            },
            after: WasmArtifactMetrics {
                raw_bytes: 90,
                gzip_bytes: 75,
                code_section_bytes: 60,
                data_section_bytes: 20,
                defined_functions: 9,
            },
        }),
    };

    let transforms = artifact_transform_provenance(&sample_request(&root, output))
        .expect("transform provenance");

    fs::remove_dir_all(&root).expect("remove root");
    assert_eq!(transforms[2].tool, "wasm-opt");
    assert_eq!(
        transforms[2].tool_sha256.as_deref(),
        Some(expected_tool_sha256.as_str())
    );
    let metrics = transforms[2].metrics.as_ref().expect("optimizer metrics");
    assert_eq!(metrics.before.code_section_bytes, 70);
    assert_eq!(metrics.after.code_section_bytes, 60);
}

#[test]
fn build_provenance_envelope_wraps_stable_payload() {
    let root = temp_dir("canic-build-provenance-envelope");
    write_sample_workspace(&root, "demo", "app");
    let output = write_sample_artifacts(&root, "app");
    let request = BuildProvenanceRequest {
        app: "demo".to_string(),
        role: "app".to_string(),
        environment: "staging".to_string(),
        build_network: BuildNetwork::Ic,
        profile: CanisterBuildProfile::Fast,
        workspace_root: root.clone(),
        config_path: root.join("apps/demo/canic.toml"),
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
    assert_eq!(envelope.target.app.as_deref(), Some("demo"));
    assert_eq!(envelope.target.role.as_deref(), Some("app"));
    assert_eq!(envelope.target.environment.as_deref(), Some("staging"));
    assert!(envelope.inputs.iter().any(|input| {
        input.kind == "build_network"
            && input.note.as_deref() == Some("environment=staging;build_network=ic")
    }));
    assert_eq!(envelope.payload_schema, build_provenance_schema());
    assert_eq!(payload.cargo.package_metadata_app, "demo");
    assert_eq!(payload.cargo.package_metadata_role, "app");
    assert!(payload.cargo.cargo_lock_sha256.is_some());
    assert_eq!(payload.protocol_profile.candid_sha256, "03".repeat(32));
    assert_eq!(
        payload.protocol_profile.protocol_profile_digest,
        "04".repeat(32)
    );
    assert_eq!(payload.artifacts.len(), 2);
    assert_eq!(payload.transforms.len(), 3);
    assert_eq!(payload.transforms[0].tool, "ic-wasm");
    assert_eq!(payload.transforms[1].tool, "ic-wasm");
    assert_eq!(payload.transforms[2].tool, "wasm-opt");
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
    assert_eq!(
        payload.transforms[2].transform,
        ArtifactTransformKindV1::Optimize
    );
    assert_eq!(
        payload.transforms[2].outcome,
        ArtifactTransformOutcomeV1::NotRequested
    );
    assert_eq!(payload.transforms[2].tool_version, None);
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

#[test]
fn build_provenance_rejects_release_optimizer_without_exact_digest() {
    let root = temp_dir("canic-build-provenance-transform-digest");
    write_sample_workspace(&root, "demo", "app");
    for tool_sha256 in [
        None,
        Some("not-a-sha256".to_string()),
        Some("ab".repeat(32)),
    ] {
        let mut output = write_sample_artifacts(&root, "app");
        output.transforms[2] = ArtifactTransformOutput {
            transform: ArtifactTransformKind::Optimize,
            tool_version: Some("wasm-opt version 108 (version_108)".to_string()),
            tool_sha256,
            outcome: ArtifactTransformOutcome::Applied,
            metrics: Some(WasmTransformMetrics {
                before: WasmArtifactMetrics {
                    raw_bytes: 10,
                    gzip_bytes: 9,
                    code_section_bytes: 8,
                    data_section_bytes: 1,
                    defined_functions: 1,
                },
                after: WasmArtifactMetrics {
                    raw_bytes: 9,
                    gzip_bytes: 8,
                    code_section_bytes: 7,
                    data_section_bytes: 1,
                    defined_functions: 1,
                },
            }),
        };

        artifact_transform_provenance(&sample_request(&root, output))
            .expect_err("non-authoritative optimizer digest must reject");
    }

    fs::remove_dir_all(root).expect("remove root");
}

#[test]
fn build_provenance_rejects_artifact_package_identity_drift() {
    let root = temp_dir("canic-build-provenance-package-drift");
    write_sample_workspace(&root, "demo", "app");
    let mut output = write_sample_artifacts(&root, "app");
    output.package_name = "other-package".to_string();
    let request = sample_request(&root, output);

    build_provenance_envelope(&request).expect_err("package identity drift must reject");

    fs::remove_dir_all(&root).expect("remove root");
}

fn sample_request(root: &Path, output: CanisterArtifactBuildOutput) -> BuildProvenanceRequest {
    BuildProvenanceRequest {
        app: "demo".to_string(),
        role: "app".to_string(),
        environment: "local".to_string(),
        build_network: BuildNetwork::Local,
        profile: CanisterBuildProfile::Fast,
        workspace_root: root.to_path_buf(),
        config_path: root.join("apps/demo/canic.toml"),
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

fn write_sample_workspace(root: &Path, app: &str, role: &str) {
    let package_dir = root.join("apps").join(app).join(role);
    fs::create_dir_all(package_dir.join("src")).expect("create package");
    fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[workspace]
members = ["apps/{app}/{role}"]
resolver = "3"
"#
        ),
    )
    .expect("write workspace manifest");
    fs::write(root.join("Cargo.lock"), "# lock\n").expect("write lock");
    fs::write(
        root.join("apps").join(app).join("canic.toml"),
        format!(
            r#"[app]
name = "{app}"

[roles.root]
kind = "root"
package = "root"

[roles.{role}]
kind = "canister"
package = "{role}"



[component_specs.default]
component_role = "{role}"
maximum_instances = 1
"#
        ),
    )
    .expect("write canic config");
    fs::write(
        package_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "canister_{app}_{role}"
version = "0.0.0"
edition = "2024"

[package.metadata.canic]
app = "{app}"
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
        package_name: format!("canister_demo_{role}"),
        package_version: "0.101.51".to_string(),
        protocol_release_identity: "0.101.51".to_string(),
        protocol_role: canic_core::ids::CanisterRole::owned(role.to_string()),
        protocol_capabilities: std::collections::BTreeSet::new(),
        artifact_root,
        wasm_path,
        wasm_gz_path,
        did_path,
        candid_sha256: [3; 32],
        protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest::from_bytes(
            [4; 32],
        ),
        transforms: vec![
            ArtifactTransformOutput {
                transform: ArtifactTransformKind::Shrink,
                tool_version: Some("ic-wasm 0.test".to_string()),
                tool_sha256: None,
                outcome: ArtifactTransformOutcome::Applied,
                metrics: None,
            },
            ArtifactTransformOutput::not_requested(ArtifactTransformKind::CandidMetadata),
            ArtifactTransformOutput::not_requested(ArtifactTransformKind::Optimize),
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
