mod environment;

use super::*;
use crate::{
    durable_io::lock_file,
    release_build::{
        finalize_release_build_from_manifest, plan_release_build_for_profile_and_network,
    },
    release_set::{
        ApplicationArtifactBuildTarget, ApplicationArtifactFileBuildOutput,
        CanicInfrastructureArtifactBuildOutput, CanicInfrastructureRole,
        compile_and_persist_application_artifact_union,
        compile_and_persist_canic_infrastructure_artifact_manifest,
        compile_and_persist_current_release_set_manifest,
        fixture::compile_and_persist_fixture_artifact_manifest,
    },
};
use canic_core::{ids::CanisterRole, role_contract::ProtocolProfileDigest};
use flate2::{Compression, GzBuilder};

#[test]
fn verified_repeat_survives_missing_diagnostics_and_rejects_tampered_output() {
    run_with_private_cargo_target(|| {
        let (root, context) = infrastructure_build_fixture();
        let directory = context.icp_root.join(".canic/build-reuse");
        let reuse = prepared_reuse(&context);
        assert!(reuse.load().unwrap().is_none());
        let release = finalize_fixture(&context);
        reuse
            .record(
                release,
                vec![
                    "app".into(),
                    "root".into(),
                    "fleet_coordinator".into(),
                    "wasm_store".into(),
                ],
            )
            .unwrap();
        assert_eq!(reuse.load().unwrap().unwrap().release_build_id, release);
        fs::remove_file(directory.join("last-input-diagnostics.json")).unwrap();
        assert_eq!(reuse.load().unwrap().unwrap().release_build_id, release);
        fs::write(directory.join("last-input-diagnostics.json"), b"invalid").unwrap();
        assert_eq!(reuse.load().unwrap().unwrap().release_build_id, release);
        fs::write(
            reuse.release_directory(release).join("artifacts/app.wasm"),
            b"tampered",
        )
        .unwrap();
        assert!(matches!(reuse.load(), Err(BuildReuseError::Evidence(_))));
        drop(reuse);
        fs::remove_dir_all(root).unwrap();
    });
}

fn prepared_reuse(context: &WorkspaceBuildContext) -> CompleteBuildReuse {
    let inputs = input_snapshot(context, &[]).unwrap();
    CompleteBuildReuse {
        input_locations: diagnostics::InputLocations::capture(context),
        diagnostics: diagnostics::InputDiagnostics::capture(context, &[], &inputs),
        record_path: context
            .icp_root
            .join(".canic/build-reuse")
            .join(format!("{}.json", inputs.digest())),
        inputs,
        tool_paths: vec![],
        _lock: lock_file(
            &context
                .icp_root
                .join(".canic/locks/complete-build-reuse.lock"),
        )
        .unwrap(),
        context: context.clone(),
    }
}

#[test]
fn rejected_build_retains_path_fingerprints_without_becoming_reuse_authority() {
    run_with_private_cargo_target(|| {
        let (root, context) = infrastructure_build_fixture();
        let reuse = prepared_reuse(&context);
        let release = finalize_fixture(&context);
        let source = context.workspace_root.join("src/lib.rs");
        let original = fs::read(&source).unwrap();
        let before_hash = file_hash(&source).unwrap();
        fs::write(&source, b"pub const EDITED: bool = true;\n").unwrap();
        let after_hash = file_hash(&source).unwrap();
        assert!(matches!(reuse.record(release, vec![]),
            Err(BuildReuseError::ChangedInput(path)) if path == source));
        let evidence_path = context
            .icp_root
            .join(".canic/build-reuse")
            .join(format!("rejected-{release}.json"));
        let bytes = fs::read(&evidence_path).unwrap();
        let evidence: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            evidence["release_build_id"],
            serde_json::to_value(release).unwrap()
        );
        assert_eq!(evidence["kind"], "input_changed");
        assert_eq!(evidence["affected_input"]["path"], source.to_str().unwrap());
        assert_eq!(
            evidence["affected_input"]["before_snapshot_value"],
            before_hash
        );
        assert_eq!(
            evidence["affected_input"]["after_snapshot_value"],
            after_hash
        );
        assert_ne!(
            evidence["before_inputs_sha256"],
            evidence["after_inputs_sha256"]
        );
        assert!(
            evidence["before_locations"]["output_roots"]["declarations"]["selected"].is_string()
        );
        assert!(!String::from_utf8_lossy(&bytes).contains("pub const EDITED"));
        assert!(!reuse.record_path.exists());
        assert!(reuse.load().unwrap().is_none());
        fs::write(&source, original).unwrap();
        reuse
            .record(
                release,
                vec![
                    "app".into(),
                    "root".into(),
                    "fleet_coordinator".into(),
                    "wasm_store".into(),
                ],
            )
            .unwrap();
        assert_eq!(
            fs::read(&evidence_path).unwrap(),
            bytes,
            "successful retry preserves rejection evidence"
        );
        fs::write(&evidence_path, b"invalid diagnostic").unwrap();
        assert_eq!(reuse.load().unwrap().unwrap().release_build_id, release);
        drop(reuse);
        fs::remove_dir_all(root).unwrap();
    });
}

#[test]
fn unavailable_rejection_diagnostics_preserve_the_typed_failure() {
    run_with_private_cargo_target(|| {
        let (root, context) = infrastructure_build_fixture();
        let reuse = prepared_reuse(&context);
        let release = finalize_fixture(&context);
        let source = context.workspace_root.join("src/lib.rs");
        fs::write(&source, b"pub const EDITED: bool = true;\n").unwrap();
        let evidence_path = context
            .icp_root
            .join(".canic/build-reuse")
            .join(format!("rejected-{release}.json"));
        fs::create_dir_all(&evidence_path).unwrap();
        assert!(matches!(reuse.record(release, vec![]),
            Err(BuildReuseError::ChangedInput(path)) if path == source));
        assert!(!reuse.record_path.exists());
        fs::remove_dir(&evidence_path).unwrap();
        #[cfg(unix)]
        {
            let destination = root.join("untouched.txt");
            fs::write(&destination, b"untouched").unwrap();
            std::os::unix::fs::symlink(&destination, &evidence_path).unwrap();
            assert!(matches!(reuse.record(release, vec![]),
                Err(BuildReuseError::ChangedInput(path)) if path == source));
            assert_eq!(fs::read(&destination).unwrap(), b"untouched");
        }
        drop(reuse);
        fs::remove_dir_all(root).unwrap();
    });
}

fn finalize_fixture(context: &WorkspaceBuildContext) -> ReleaseBuildId {
    let root = &context.icp_root;
    let release =
        plan_release_build_for_profile_and_network(root, context.profile, context.build_network)
            .unwrap()
            .record
            .release_build_id;
    let config = AppConfigSnapshot::load(&context.config_path).unwrap();
    let topology = config.component_topology();
    let (wasm_path, wasm_gz_path) = artifact(root, "app", release);
    let application = compile_and_persist_application_artifact_union(
        root,
        topology,
        release,
        &[ApplicationArtifactBuildTarget {
            role: CanisterRole::owned("app".into()),
            package: "app-package".into(),
            wasm_relative_path: wasm_path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
            wasm_gz_relative_path: wasm_gz_path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
        }],
        &[ApplicationArtifactFileBuildOutput {
            role: CanisterRole::owned("app".into()),
            package: "app-package".into(),
            release_build_id: release,
            wasm_path,
            wasm_gz_path,
            candid_sha256: [3; 32],
            protocol_profile_digest: ProtocolProfileDigest::from_bytes([4; 32]),
        }],
    )
    .unwrap();
    let outputs = [
        CanicInfrastructureRole::FleetCoordinator,
        CanicInfrastructureRole::FleetSubnetRoot,
        CanicInfrastructureRole::WasmStore,
    ]
    .map(|role| {
        let (wasm_path, wasm_gz_path) = artifact(root, role.as_str(), release);
        CanicInfrastructureArtifactBuildOutput {
            role,
            package: "canic-control-plane".into(),
            protocol_release_identity: env!("CARGO_PKG_VERSION").into(),
            protocol_role: CanisterRole::owned(role.protocol_role_name().into()),
            protocol_capabilities: BTreeSet::new(),
            release_build_id: release,
            wasm_path,
            wasm_gz_path,
            candid_sha256: [3; 32],
            protocol_profile_digest: ProtocolProfileDigest::from_bytes([4; 32]),
        }
    });
    let infrastructure =
        compile_and_persist_canic_infrastructure_artifact_manifest(root, release, &outputs)
            .unwrap();
    let fixtures =
        compile_and_persist_fixture_artifact_manifest(root, topology, release, &[]).unwrap();
    let current = compile_and_persist_current_release_set_manifest(
        root,
        topology,
        release,
        &application,
        &infrastructure,
        &fixtures,
    )
    .unwrap();
    finalize_release_build_from_manifest(root, release, &current.path).unwrap();
    release
}

fn artifact(root: &Path, name: &str, release: ReleaseBuildId) -> (PathBuf, PathBuf) {
    let directory = root
        .join(".canic/release-builds")
        .join(release.to_string())
        .join("artifacts");
    fs::create_dir_all(&directory).unwrap();
    // Byte-admission fixture only: no executable IC behavior is claimed.
    let mut bytes = b"\0asm".to_vec();
    bytes.extend_from_slice(release.to_string().as_bytes());
    let path = directory.join(format!("{name}.wasm"));
    let gzip_path = directory.join(format!("{name}.wasm.gz"));
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::fast());
    encoder.write_all(&bytes).unwrap();
    fs::write(&path, &bytes).unwrap();
    fs::write(&gzip_path, encoder.finish().unwrap()).unwrap();
    (path, gzip_path)
}
