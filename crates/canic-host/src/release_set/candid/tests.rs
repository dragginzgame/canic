//! Selected-release resolution and rejection tests without network effects.

use super::*;
use crate::{
    release_build::{finalize_release_build_from_manifest, plan_release_build},
    release_set::{
        ApplicationArtifactEntry, ApplicationArtifactUnion, CanicInfrastructureArtifactEntry,
        CanicInfrastructureArtifactManifest, CanicInfrastructureRole, CurrentReleaseSetManifest,
    },
    test_support::temp_dir,
};
use canic_core::role_contract::{RoleCapabilityKey, derive_protocol_profile_hashes};
use std::{collections::BTreeSet, fs};

struct Fixture {
    root: PathBuf,
    release: ReleaseBuildId,
}

impl Fixture {
    fn new(finalize: bool) -> Self {
        let root = temp_dir("admission-release-protocol");
        let plan = plan_release_build(&root).expect("plan release");
        let release = plan.record.release_build_id;
        let entries = [
            CanicInfrastructureRole::FleetCoordinator,
            CanicInfrastructureRole::FleetSubnetRoot,
            CanicInfrastructureRole::WasmStore,
        ]
        .into_iter()
        .map(|role| {
            let protocol_role = role.protocol_role_name().into();
            let capabilities = BTreeSet::from([RoleCapabilityKey::Runtime]);
            let candid = b"service : {}\n";
            let hashes = derive_protocol_profile_hashes(
                &plan.record.builder_version,
                &protocol_role,
                &capabilities,
                candid,
            );
            let wasm_relative_path = format!(
                ".canic/release-builds/{release}/artifacts/{0}/{0}.wasm",
                role.as_str(),
            );
            let did = root.join(&wasm_relative_path).with_extension("did");
            fs::create_dir_all(did.parent().expect("parent")).expect("mkdir");
            fs::write(did, candid).expect("write sidecar");
            CanicInfrastructureArtifactEntry {
                role,
                package: role.as_str().to_string(),
                protocol_release_identity: plan.record.builder_version.clone(),
                protocol_role,
                protocol_capabilities: capabilities,
                release_build_id: release,
                wasm_gz_relative_path: format!("{wasm_relative_path}.gz"),
                wasm_relative_path,
                wasm_size_bytes: 1,
                wasm_gz_size_bytes: 1,
                wasm_sha256_hex: "01".repeat(32),
                wasm_gz_sha256_hex: "02".repeat(32),
                candid_sha256: hashes.candid_sha256,
                protocol_profile_digest: hashes.protocol_profile_digest,
            }
        })
        .collect();
        let manifest = CanicInfrastructureArtifactManifest {
            release_build_id: release,
            entries,
        };
        let directory = root.join(".canic/release-builds").join(release.to_string());
        fs::write(
            directory.join("infrastructure-artifact-manifest.json"),
            manifest
                .canonical_bytes()
                .expect("canonical infrastructure"),
        )
        .expect("write manifest");
        let application_digest = write_application(&root, release, &directory);
        let current = CurrentReleaseSetManifest {
            application_artifact_union_sha256: application_digest,
            build_network: plan.record.build_network,
            fixture_artifact_manifest_sha256: [4; 32],
            infrastructure_artifact_manifest_sha256: manifest.digest().expect("digest"),
            release_build_id: release,
            schema_version: CurrentReleaseSetManifest::SCHEMA_VERSION,
            transition_mode: crate::release_set::ReleaseTransitionMode::ReinstallOnly,
        };
        let current_path = directory.join("current-release-set-manifest.json");
        fs::write(
            &current_path,
            current.canonical_bytes().expect("canonical current"),
        )
        .expect("write current manifest");
        if finalize {
            finalize_release_build_from_manifest(&root, release, &current_path).expect("finalize");
        }
        Self { root, release }
    }

    fn directory(&self) -> PathBuf {
        self.root
            .join(".canic/release-builds")
            .join(self.release.to_string())
    }

    fn sidecar(&self) -> PathBuf {
        self.directory().join("artifacts/app/app.did")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn write_application(root: &Path, release: ReleaseBuildId, directory: &Path) -> [u8; 32] {
    let topology = canic_core::bootstrap::parse_config_model(
        r#"
[app]
name = "demo"
[roles.root]
kind = "root"
[roles.app]
kind = "canister"
package = "app"
[component_specs.app]
component_role = "app"
maximum_instances = 1
"#,
    )
    .unwrap()
    .compile_component_topology()
    .unwrap();
    let candid = b"service : { submit : (blob) -> (); }\n";
    let wasm = format!(".canic/release-builds/{release}/artifacts/app/app.wasm");
    let did = root.join(&wasm).with_extension("did");
    fs::create_dir_all(did.parent().unwrap()).unwrap();
    fs::write(&did, candid).unwrap();
    let application = ApplicationArtifactUnion {
        release_build_id: release,
        fleet_component_topology_digest: topology.digest().unwrap(),
        entries: vec![ApplicationArtifactEntry {
            role: "app".into(),
            package: "app".into(),
            release_build_id: release,
            wasm_gz_relative_path: format!("{wasm}.gz"),
            wasm_relative_path: wasm,
            wasm_size_bytes: 1,
            wasm_gz_size_bytes: 1,
            wasm_sha256_hex: "01".repeat(32),
            wasm_gz_sha256_hex: "02".repeat(32),
            candid_sha256: Sha256::digest(candid).into(),
            protocol_profile_digest: canic_core::role_contract::ProtocolProfileDigest::from_bytes(
                [7; 32],
            ),
        }],
    };
    fs::write(
        directory.join("application-artifact-union.json"),
        application.canonical_bytes(&topology).unwrap(),
    )
    .unwrap();
    application.digest(&topology).unwrap()
}

#[test]
fn exact_build_inspects_application_and_infrastructure_without_workspace_or_live_state() {
    let fixture = Fixture::new(true);
    for role in ["app", "root", "fleet_coordinator", "wasm_store"] {
        let built = load_built_candid(&fixture.root, fixture.release, role).unwrap();
        assert_eq!(built.release_build_id, fixture.release);
        assert_eq!(built.candid.as_bytes(), fs::read(&built.path).unwrap());
        assert!(built.path.starts_with(fixture.directory()));
    }
    assert!(matches!(
        load_built_candid(&fixture.root, fixture.release, "unknown"),
        Err(BuiltCandidError::MissingRole(_))
    ));
}

#[test]
fn selected_build_rejects_unfinished_build_and_altered_finalization() {
    let fixture = Fixture::new(false);
    assert!(matches!(
        load_built_candid(&fixture.root, fixture.release, "app"),
        Err(BuiltCandidError::ReleaseBuild(_))
    ));
    let fixture = Fixture::new(true);
    let path = fixture
        .directory()
        .join("current-release-set-manifest.json");
    let mut current: CurrentReleaseSetManifest =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    current.application_artifact_union_sha256 = [9; 32];
    fs::write(&path, current.canonical_bytes().unwrap()).unwrap();
    assert!(matches!(
        load_built_candid(&fixture.root, fixture.release, "app"),
        Err(BuiltCandidError::ReleaseBuild(
            ReleaseBuildPlanError::ManifestDigestMismatch { .. }
        ))
    ));
}

#[test]
fn selected_build_rejects_changed_child_manifests_and_declaration() {
    for child in [
        "application-artifact-union.json",
        "infrastructure-artifact-manifest.json",
    ] {
        let fixture = Fixture::new(true);
        let path = fixture.directory().join(child);
        // Keep the child canonical and internally valid while changing its bound content.
        let bytes = fs::read(&path).unwrap();
        let changed = if child.starts_with("application") {
            let mut child: ApplicationArtifactUnion = serde_json::from_slice(&bytes).unwrap();
            child.entries[0].candid_sha256 = [9; 32];
            serde_json::to_vec(&child).unwrap()
        } else {
            let mut child: CanicInfrastructureArtifactManifest =
                serde_json::from_slice(&bytes).unwrap();
            child.entries[0].candid_sha256 = [9; 32];
            child.canonical_bytes().unwrap()
        };
        fs::write(path, changed).unwrap();
        assert!(matches!(
            load_built_candid(&fixture.root, fixture.release, "app"),
            Err(BuiltCandidError::ManifestDigestMismatch)
        ));
    }
    let fixture = Fixture::new(true);
    fs::write(fixture.sidecar(), b"service : {}\n").unwrap();
    assert!(matches!(
        load_built_candid(&fixture.root, fixture.release, "app"),
        Err(BuiltCandidError::CandidDigestMismatch(_))
    ));
    fs::remove_file(fixture.sidecar()).unwrap();
    assert!(matches!(
        load_built_candid(&fixture.root, fixture.release, "app"),
        Err(BuiltCandidError::UnsafeCandid(_))
    ));
}

#[cfg(unix)]
#[test]
fn selected_build_rejects_symlinked_sidecars_and_escaping_parents() {
    let fixture = Fixture::new(true);
    let other = Fixture::new(true);
    fs::remove_file(fixture.sidecar()).unwrap();
    std::os::unix::fs::symlink(other.sidecar(), fixture.sidecar()).unwrap();
    assert!(matches!(
        load_built_candid(&fixture.root, fixture.release, "app"),
        Err(BuiltCandidError::UnsafeCandid(_))
    ));
    fs::remove_dir_all(fixture.sidecar().parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(
        other.sidecar().parent().unwrap(),
        fixture.sidecar().parent().unwrap(),
    )
    .unwrap();
    assert!(matches!(
        load_built_candid(&fixture.root, fixture.release, "app"),
        Err(BuiltCandidError::UnsafeCandid(_))
    ));
}
