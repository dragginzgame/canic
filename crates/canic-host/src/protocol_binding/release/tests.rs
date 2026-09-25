//! Selected-release resolution and rejection tests without network effects.

use super::*;
use crate::{
    release_build::{finalize_release_build_from_manifest, plan_release_build},
    release_set::{
        CanicInfrastructureArtifactEntry, CanicInfrastructureArtifactManifest,
        CanicInfrastructureRole, CurrentReleaseSetManifest,
    },
    test_support::temp_dir,
};
use canic_core::role_contract::{RoleCapabilityKey, derive_protocol_profile_hashes};
use std::{collections::BTreeSet, fs};

struct Fixture {
    root: PathBuf,
    release: ReleaseBuildId,
    manifest: CanicInfrastructureArtifactManifest,
}

impl Fixture {
    fn new() -> Self {
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
        let current = CurrentReleaseSetManifest {
            application_artifact_union_sha256: [3; 32],
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
        finalize_release_build_from_manifest(&root, release, &current_path).expect("finalize");
        Self {
            root,
            release,
            manifest,
        }
    }

    fn entry(&self, index: usize) -> RegistryEntry {
        let artifact = &self.manifest.entries[index];
        RegistryEntry {
            pid: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
            role: Some(artifact.protocol_role.to_string()),
            parent_pid: None,
            module_hash: Some(artifact.wasm_sha256_hex.clone()),
            protocol_binding: Some(RegistryProtocolBinding {
                release_identity: artifact.protocol_release_identity.clone(),
                role: artifact.protocol_role.clone(),
                capabilities: artifact.protocol_capabilities.clone(),
                candid_sha256: artifact.candid_sha256,
                protocol_profile_digest: artifact.protocol_profile_digest,
            }),
        }
    }

    fn sidecar(&self, index: usize) -> PathBuf {
        self.root
            .join(&self.manifest.entries[index].wasm_relative_path)
            .with_extension("did")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("remove fixture");
    }
}

#[test]
fn coordinator_and_roots_resolve_from_finalized_release_without_environment_sidecars() {
    let fixture = Fixture::new();
    for index in 0..fixture.manifest.entries.len() {
        let entry = fixture.entry(index);
        let resolved =
            resolve_release_registry_protocol_binding(&fixture.root, fixture.release, &entry)
                .expect("resolve retained role");
        assert_eq!(resolved.candid_path(), fixture.sidecar(index));
        assert_eq!(Some(resolved.binding()), entry.protocol_binding.as_ref());
    }
}

#[test]
fn terminal_role_module_and_protocol_authority_must_match_selected_release() {
    let fixture = Fixture::new();
    let original = fixture.entry(0);
    let mut cases = Vec::new();
    let mut entry = original.clone();
    entry.module_hash = Some("09".repeat(32));
    cases.push(entry);
    let mut entry = original.clone();
    entry.role = Some("root".into());
    cases.push(entry);
    let mut entry = original.clone();
    entry
        .protocol_binding
        .as_mut()
        .expect("binding")
        .release_identity
        .push_str("-changed");
    cases.push(entry);
    let mut entry = original;
    entry.protocol_binding = None;
    cases.push(entry);
    for entry in cases {
        assert!(matches!(
            resolve_release_registry_protocol_binding(&fixture.root, fixture.release, &entry),
            Err(ReleaseProtocolBindingError::ArtifactBindingMismatch { .. })
        ));
    }
}

#[test]
fn missing_and_modified_selected_sidecars_reject_before_transport() {
    let fixture = Fixture::new();
    let entry = fixture.entry(0);
    fs::write(fixture.sidecar(0), b"service : { changed : () -> (); }\n").expect("tamper");
    assert!(matches!(
        resolve_release_registry_protocol_binding(&fixture.root, fixture.release, &entry),
        Err(ReleaseProtocolBindingError::Binding(
            ProtocolBindingError::CandidHashMismatch { .. }
        ))
    ));
    fs::remove_file(fixture.sidecar(0)).expect("remove");
    assert!(matches!(
        resolve_release_registry_protocol_binding(&fixture.root, fixture.release, &entry),
        Err(ReleaseProtocolBindingError::Binding(
            ProtocolBindingError::MissingCandid { .. }
        ))
    ));
}

#[test]
fn changed_infrastructure_and_finalization_manifests_reject() {
    let mut fixture = Fixture::new();
    let entry = fixture.entry(0);
    fixture.manifest.entries[0].package.push_str("-changed");
    let directory = fixture
        .root
        .join(".canic/release-builds")
        .join(fixture.release.to_string());
    fs::write(
        directory.join("infrastructure-artifact-manifest.json"),
        fixture
            .manifest
            .canonical_bytes()
            .expect("canonical modified manifest"),
    )
    .expect("tamper");
    assert!(matches!(
        resolve_release_registry_protocol_binding(&fixture.root, fixture.release, &entry),
        Err(ReleaseProtocolBindingError::ManifestDigestMismatch)
    ));

    let path = directory.join("current-release-set-manifest.json");
    let mut current: CurrentReleaseSetManifest =
        serde_json::from_slice(&fs::read(&path).expect("read")).expect("decode");
    current.infrastructure_artifact_manifest_sha256 = fixture.manifest.digest().expect("digest");
    fs::write(path, current.canonical_bytes().expect("canonical")).expect("tamper current");
    assert!(matches!(
        resolve_release_registry_protocol_binding(&fixture.root, fixture.release, &entry),
        Err(ReleaseProtocolBindingError::ReleaseBuild(
            ReleaseBuildPlanError::ManifestDigestMismatch { .. }
        ))
    ));
}

#[cfg(unix)]
#[test]
fn linked_sidecar_rejects_even_with_matching_bytes() {
    let fixture = Fixture::new();
    let entry = fixture.entry(0);
    let path = fixture.sidecar(0);
    let moved = path.with_extension("saved");
    fs::rename(&path, &moved).expect("move");
    std::os::unix::fs::symlink(moved, path).expect("link");
    assert!(matches!(
        resolve_release_registry_protocol_binding(&fixture.root, fixture.release, &entry),
        Err(ReleaseProtocolBindingError::Binding(
            ProtocolBindingError::UnsafeCandid { .. }
        ))
    ));
}
