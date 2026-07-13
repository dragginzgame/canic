//! Module: release_set::manifest
//!
//! Responsibility: define, validate, load, and persist root release-set manifests.
//! Does not own: artifact bytes, ICP calls, or bootstrap sequencing.
//! Boundary: admits one exact identity and artifact-shape contract for every consumer.

use crate::{
    canister_build::CurrentCanisterArtifactBuildOutput, durable_io::write_bytes,
    release_set::build_release_set_entry,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use canic_core::{
    CANIC_WASM_CHUNK_BYTES, bootstrap::compiled::validate_canister_role_name,
    cdk::utils::hash::decode_hex,
};
use serde::{Deserialize, Serialize};

const SHA_256_BYTES: usize = 32;

///
/// RootReleaseSetManifest
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootReleaseSetManifest {
    pub release_version: String,
    pub entries: Vec<ReleaseSetEntry>,
}

///
/// ReleaseSetEntry
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReleaseSetEntry {
    pub role: String,
    pub template_id: String,
    pub artifact_relative_path: String,
    pub payload_size_bytes: u64,
    pub payload_sha256_hex: String,
    pub chunk_size_bytes: u64,
    pub chunk_sha256_hex: Vec<String>,
}

/// One required complete-build target and its exact admitted gzip path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootReleaseSetBuildTarget {
    pub(crate) role: String,
    pub(crate) expected_wasm_gz_path: PathBuf,
    pub(crate) publish_entry: bool,
}

/// Immutable manifest inputs derived before the complete build starts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootReleaseSetBuildSnapshot {
    pub(crate) icp_root: PathBuf,
    pub(crate) manifest_path: PathBuf,
    pub(crate) release_version: String,
    pub(crate) targets: Vec<RootReleaseSetBuildTarget>,
}

/// Validate the canonical manifest contract shared by writers, loaders, and
/// staging.
pub fn validate_root_release_set_manifest(
    manifest: &RootReleaseSetManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    if manifest.release_version.trim().is_empty() {
        return Err("release-set manifest version must not be empty".into());
    }

    let mut roles = BTreeSet::new();
    for entry in &manifest.entries {
        validate_canister_role_name(&entry.role)
            .map_err(|issue| format!("invalid release-set role {:?}: {issue}", entry.role))?;
        if !roles.insert(entry.role.as_str()) {
            return Err(format!("duplicate release-set role: {}", entry.role).into());
        }

        let expected_template_id = format!("embedded:{}", entry.role);
        if entry.template_id != expected_template_id {
            return Err(format!(
                "release-set template identity mismatch for role {}: expected {}",
                entry.role, expected_template_id
            )
            .into());
        }

        if entry.payload_size_bytes == 0 {
            return Err(format!(
                "release-set payload size must be nonzero for role {}",
                entry.role
            )
            .into());
        }

        let canonical_chunk_size = u64::try_from(CANIC_WASM_CHUNK_BYTES)?;
        if entry.chunk_size_bytes != canonical_chunk_size {
            return Err(format!(
                "release-set chunk size must be {canonical_chunk_size} for role {}",
                entry.role
            )
            .into());
        }

        validate_sha256_hex(
            &entry.payload_sha256_hex,
            &format!("payload hash for role {}", entry.role),
        )?;

        let expected_chunk_count =
            usize::try_from(entry.payload_size_bytes.div_ceil(entry.chunk_size_bytes))?;
        if entry.chunk_sha256_hex.len() != expected_chunk_count {
            return Err(format!(
                "release-set chunk count must be {expected_chunk_count} for role {}",
                entry.role
            )
            .into());
        }
        for (chunk_index, chunk_hash) in entry.chunk_sha256_hex.iter().enumerate() {
            validate_sha256_hex(
                chunk_hash,
                &format!("chunk hash {chunk_index} for role {}", entry.role),
            )?;
        }
    }

    Ok(())
}

fn validate_sha256_hex(value: &str, field: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = decode_hex(value).map_err(|error| format!("invalid {field}: {error}"))?;
    if bytes.len() != SHA_256_BYTES {
        return Err(format!(
            "invalid {field}: expected {SHA_256_BYTES} bytes, got {}",
            bytes.len()
        )
        .into());
    }
    Ok(())
}

// Publish one manifest only from the exact outputs returned by this complete build.
pub fn emit_root_release_set_manifest_from_build(
    snapshot: &RootReleaseSetBuildSnapshot,
    outputs: &[CurrentCanisterArtifactBuildOutput],
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut expected = BTreeMap::<&str, &RootReleaseSetBuildTarget>::new();
    for target in &snapshot.targets {
        if expected.insert(&target.role, target).is_some() {
            return Err(format!("duplicate required build target: {}", target.role).into());
        }
    }

    let mut current = BTreeMap::<&str, &CurrentCanisterArtifactBuildOutput>::new();
    for output in outputs {
        if current.insert(&output.role, output).is_some() {
            return Err(format!("duplicate current build output: {}", output.role).into());
        }
        if !expected.contains_key(output.role.as_str()) {
            return Err(format!("unexpected current build output: {}", output.role).into());
        }
    }

    let mut entries = Vec::new();
    for target in &snapshot.targets {
        let output = current
            .get(target.role.as_str())
            .ok_or_else(|| format!("missing current build output: {}", target.role))?;
        if output.output.wasm_gz_path != target.expected_wasm_gz_path {
            return Err(format!(
                "current build output path mismatch for {}: expected {}, got {}",
                target.role,
                target.expected_wasm_gz_path.display(),
                output.output.wasm_gz_path.display()
            )
            .into());
        }
        if target.publish_entry {
            entries.push(build_release_set_entry(
                &snapshot.icp_root,
                &target.role,
                &output.output.wasm_gz_path,
            )?);
        }
    }

    let manifest = RootReleaseSetManifest {
        release_version: snapshot.release_version.clone(),
        entries,
    };
    validate_root_release_set_manifest(&manifest)?;
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    write_bytes(&snapshot.manifest_path, &bytes)?;
    Ok(snapshot.manifest_path.clone())
}

// Load one previously emitted root release-set manifest from disk.
pub fn load_root_release_set_manifest(
    manifest_path: &Path,
) -> Result<RootReleaseSetManifest, Box<dyn std::error::Error>> {
    let source = fs::read(manifest_path)?;
    let manifest = serde_json::from_slice(&source)?;
    validate_root_release_set_manifest(&manifest)?;
    Ok(manifest)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        canister_build::{CanisterArtifactBuildOutput, CurrentCanisterArtifactBuildOutput},
        test_support::temp_dir,
    };
    use flate2::{Compression, write::GzEncoder};
    use std::io::Write;

    fn manifest() -> RootReleaseSetManifest {
        RootReleaseSetManifest {
            release_version: "test-version".to_string(),
            entries: vec![ReleaseSetEntry {
                role: "app".to_string(),
                template_id: "embedded:app".to_string(),
                artifact_relative_path: ".icp/local/canisters/app/app.wasm.gz".to_string(),
                payload_size_bytes: 128,
                payload_sha256_hex: "00".repeat(32),
                chunk_size_bytes: 1_048_576,
                chunk_sha256_hex: vec!["00".repeat(32)],
            }],
        }
    }

    #[test]
    fn release_set_manifest_identity_accepts_canonical_roles() {
        for role in ["a", "app", "app2", "user_hub", "scale_replica", "role_2"] {
            let mut manifest = manifest();
            manifest.entries[0].role = role.to_string();
            manifest.entries[0].template_id = format!("embedded:{role}");

            assert!(validate_root_release_set_manifest(&manifest).is_ok());
        }
    }

    #[test]
    fn release_set_manifest_identity_rejects_empty_version_and_role() {
        let mut missing_version = manifest();
        missing_version.release_version.clear();
        let mut missing_role = manifest();
        missing_role.entries[0].role.clear();

        assert!(validate_root_release_set_manifest(&missing_version).is_err());
        assert!(validate_root_release_set_manifest(&missing_role).is_err());
    }

    #[test]
    fn release_set_manifest_identity_rejects_duplicate_role() {
        let mut manifest = manifest();
        manifest.entries.push(manifest.entries[0].clone());

        assert!(validate_root_release_set_manifest(&manifest).is_err());
    }

    #[test]
    fn release_set_manifest_identity_rejects_unadmitted_roles() {
        let overlong = "a".repeat(canic_core::bootstrap::compiled::NAME_MAX_BYTES + 1);
        for role in [
            "-app",
            "App",
            "_app",
            "1app",
            "user-hub",
            "app_",
            "app__worker",
            "../sentinel",
            "app/name",
            "app.name",
            "app name",
            "café",
            &overlong,
        ] {
            let mut manifest = manifest();
            manifest.entries[0].role = role.to_string();
            manifest.entries[0].template_id = format!("embedded:{role}");

            assert!(validate_root_release_set_manifest(&manifest).is_err());
        }
    }

    #[test]
    fn release_set_manifest_identity_rejects_template_role_mismatch() {
        let mut manifest = manifest();
        manifest.entries[0].template_id = "embedded:other".to_string();

        assert!(validate_root_release_set_manifest(&manifest).is_err());
    }

    #[test]
    fn release_set_manifest_artifact_shape_rejects_zero_payload_and_wrong_chunk_size() {
        let mut zero_payload = manifest();
        zero_payload.entries[0].payload_size_bytes = 0;
        let mut wrong_chunk_size = manifest();
        wrong_chunk_size.entries[0].chunk_size_bytes -= 1;

        assert!(validate_root_release_set_manifest(&zero_payload).is_err());
        assert!(validate_root_release_set_manifest(&wrong_chunk_size).is_err());
    }

    #[test]
    fn release_set_manifest_artifact_shape_rejects_impossible_chunk_count() {
        let mut manifest = manifest();
        manifest.entries[0].chunk_sha256_hex.clear();

        assert!(validate_root_release_set_manifest(&manifest).is_err());
    }

    #[test]
    fn release_set_manifest_artifact_shape_rejects_malformed_hashes() {
        let mut payload_hash = manifest();
        payload_hash.entries[0].payload_sha256_hex = "00".to_string();
        let mut chunk_hash = manifest();
        chunk_hash.entries[0].chunk_sha256_hex[0] = "not-hex".to_string();

        assert!(validate_root_release_set_manifest(&payload_hash).is_err());
        assert!(validate_root_release_set_manifest(&chunk_hash).is_err());
    }

    #[test]
    fn complete_build_manifest_preserves_snapshot_order() {
        let root = temp_dir("canic-complete-build-manifest-order");
        let snapshot = build_snapshot(&root, &["user_hub", "app"]);
        write_gzip_wasm(&expected_gzip_path(&root, "user_hub"));
        write_gzip_wasm(&expected_gzip_path(&root, "app"));
        let outputs = vec![
            build_output(&root, "app"),
            build_output(&root, "root"),
            build_output(&root, "user_hub"),
        ];

        let manifest_path =
            emit_root_release_set_manifest_from_build(&snapshot, &outputs).expect("emit manifest");
        let manifest = load_root_release_set_manifest(&manifest_path).expect("load manifest");

        fs::remove_dir_all(&root).expect("remove temp root");
        assert_eq!(
            manifest
                .entries
                .iter()
                .map(|entry| entry.role.as_str())
                .collect::<Vec<_>>(),
            ["user_hub", "app"]
        );
    }

    #[test]
    fn rejected_complete_build_outputs_leave_existing_manifest_unchanged() {
        let root = temp_dir("canic-complete-build-manifest-rejection");
        let snapshot = build_snapshot(&root, &["app"]);
        fs::create_dir_all(snapshot.manifest_path.parent().expect("manifest parent"))
            .expect("create manifest parent");
        let previous = b"previous manifest bytes";

        let root_output = build_output(&root, "root");
        let app_output = build_output(&root, "app");
        let wrong_path_output = CurrentCanisterArtifactBuildOutput {
            role: "app".to_string(),
            output: CanisterArtifactBuildOutput {
                wasm_gz_path: root.join("wrong/app.wasm.gz"),
                ..app_output.output.clone()
            },
        };
        let cases = vec![
            vec![root_output.clone()],
            vec![root_output.clone(), app_output.clone(), app_output.clone()],
            vec![
                root_output.clone(),
                app_output.clone(),
                build_output(&root, "unexpected"),
            ],
            vec![root_output.clone(), wrong_path_output],
            vec![root_output, app_output],
        ];

        for outputs in cases {
            fs::write(&snapshot.manifest_path, previous).expect("restore previous manifest");
            assert!(emit_root_release_set_manifest_from_build(&snapshot, &outputs).is_err());
            assert_eq!(
                fs::read(&snapshot.manifest_path).expect("read preserved manifest"),
                previous
            );
        }

        fs::remove_dir_all(&root).expect("remove temp root");
    }

    fn build_snapshot(root: &Path, release_roles: &[&str]) -> RootReleaseSetBuildSnapshot {
        let mut targets = vec![RootReleaseSetBuildTarget {
            role: "root".to_string(),
            expected_wasm_gz_path: expected_gzip_path(root, "root"),
            publish_entry: false,
        }];
        targets.extend(release_roles.iter().map(|role| RootReleaseSetBuildTarget {
            role: (*role).to_string(),
            expected_wasm_gz_path: expected_gzip_path(root, role),
            publish_entry: true,
        }));
        RootReleaseSetBuildSnapshot {
            icp_root: root.to_path_buf(),
            manifest_path: root.join(".icp/local/canisters/root/root.release-set.json"),
            release_version: "test-version".to_string(),
            targets,
        }
    }

    fn build_output(root: &Path, role: &str) -> CurrentCanisterArtifactBuildOutput {
        let artifact_root = root.join(".icp/local/canisters").join(role);
        CurrentCanisterArtifactBuildOutput {
            role: role.to_string(),
            output: CanisterArtifactBuildOutput {
                wasm_path: artifact_root.join(format!("{role}.wasm")),
                wasm_gz_path: artifact_root.join(format!("{role}.wasm.gz")),
                did_path: artifact_root.join(format!("{role}.did")),
                artifact_root,
            },
        }
    }

    fn expected_gzip_path(root: &Path, role: &str) -> PathBuf {
        root.join(".icp/local/canisters")
            .join(role)
            .join(format!("{role}.wasm.gz"))
    }

    fn write_gzip_wasm(path: &Path) {
        fs::create_dir_all(path.parent().expect("artifact parent"))
            .expect("create artifact parent");
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"\0asm\x01\0\0\0").expect("encode wasm");
        fs::write(path, encoder.finish().expect("finish gzip")).expect("write gzip wasm");
    }
}
