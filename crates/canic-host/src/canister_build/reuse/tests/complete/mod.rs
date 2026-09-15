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
        let inputs = input_snapshot(&context, &[]).unwrap();
        let directory = context.icp_root.join(".canic/build-reuse");
        let reuse = CompleteBuildReuse {
            diagnostics: diagnostics::InputDiagnostics::capture(&context, &[], &inputs),
            record_path: directory.join(format!("{}.json", inputs.digest())),
            inputs,
            tool_paths: vec![],
            _lock: lock_file(
                &context
                    .icp_root
                    .join(".canic/locks/complete-build-reuse.lock"),
            )
            .unwrap(),
            context: context.clone(),
        };
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
