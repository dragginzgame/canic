//! Selected-release resolution and rejection tests without network effects.

use super::*;
use crate::{
    release_build::{finalize_release_build_from_manifest, plan_release_build},
    release_set::{
        ApplicationArtifactUnion, CanicInfrastructureArtifactEntry,
        CanicInfrastructureArtifactManifest, CanicInfrastructureRole, CurrentReleaseSetManifest,
    },
    test_support::temp_dir,
};
use canic_core::role_contract::{RoleCapabilityKey, derive_protocol_profile_hashes};
use std::{collections::BTreeSet, fs};

const WIRE_SCHEMA: &[u8] = b"native schema fixture bytes";

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
                WIRE_SCHEMA,
            );
            let wasm_relative_path = format!(
                ".canic/release-builds/{release}/artifacts/{0}/{0}.wasm",
                role.as_str(),
            );
            let did = root.join(&wasm_relative_path).with_extension("did");
            fs::create_dir_all(did.parent().expect("parent")).expect("mkdir");
            fs::write(&did, candid).expect("write sidecar");
            fs::write(did.with_extension("wire.json"), WIRE_SCHEMA).unwrap();
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
                wire_schema_sha256: Sha256::digest(WIRE_SCHEMA).into(),
                candid_sha256: Sha256::digest(candid).into(),
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
        let application_digest = crate::release_set::test_artifact::write_declaration_application(
            &root,
            release,
            &directory,
            WIRE_SCHEMA,
        );
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
        self.directory().join("artifacts/app/app.wire.json")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn exact_build_inspects_application_and_infrastructure_without_workspace_or_live_state() {
    let fixture = Fixture::new(true);
    for role in ["app", "root", "fleet_coordinator", "wasm_store"] {
        let built = load_built_wire_schema(&fixture.root, fixture.release, role).unwrap();
        assert_eq!(built.release_build_id, fixture.release);
        assert_eq!(built.schema, fs::read(&built.path).unwrap());
        assert!(built.path.starts_with(fixture.directory()));
    }
    assert!(matches!(
        load_built_wire_schema(&fixture.root, fixture.release, "unknown"),
        Err(BuiltDeclarationError::MissingRole(_))
    ));
}

#[test]
fn selected_build_rejects_unfinished_build_and_altered_finalization() {
    let fixture = Fixture::new(false);
    assert!(matches!(
        load_built_wire_schema(&fixture.root, fixture.release, "app"),
        Err(BuiltDeclarationError::ReleaseBuild(_))
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
        load_built_wire_schema(&fixture.root, fixture.release, "app"),
        Err(BuiltDeclarationError::ReleaseBuild(
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
            child.entries[0].wire_schema_sha256 = [9; 32];
            serde_json::to_vec(&child).unwrap()
        } else {
            let mut child: CanicInfrastructureArtifactManifest =
                serde_json::from_slice(&bytes).unwrap();
            child.entries[0].wire_schema_sha256 = [9; 32];
            child.canonical_bytes().unwrap()
        };
        fs::write(path, changed).unwrap();
        assert!(matches!(
            load_built_wire_schema(&fixture.root, fixture.release, "app"),
            Err(BuiltDeclarationError::ManifestDigestMismatch)
        ));
    }
    let fixture = Fixture::new(true);
    fs::write(fixture.sidecar(), b"changed schema").unwrap();
    assert!(matches!(
        load_built_wire_schema(&fixture.root, fixture.release, "app"),
        Err(BuiltDeclarationError::DigestMismatch(_))
    ));
    fs::remove_file(fixture.sidecar()).unwrap();
    assert!(matches!(
        load_built_wire_schema(&fixture.root, fixture.release, "app"),
        Err(BuiltDeclarationError::UnsafeDeclaration(_))
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
        load_built_wire_schema(&fixture.root, fixture.release, "app"),
        Err(BuiltDeclarationError::UnsafeDeclaration(_))
    ));
    fs::remove_dir_all(fixture.sidecar().parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(
        other.sidecar().parent().unwrap(),
        fixture.sidecar().parent().unwrap(),
    )
    .unwrap();
    assert!(matches!(
        load_built_wire_schema(&fixture.root, fixture.release, "app"),
        Err(BuiltDeclarationError::UnsafeDeclaration(_))
    ));
}

#[test]
fn wire_schemas_use_exact_release_authority_and_refuse_sidecar_changes() {
    let fixture = Fixture::new(true);
    for role in ["app", "root", "fleet_coordinator", "wasm_store"] {
        let schema = load_built_wire_schema(&fixture.root, fixture.release, role).unwrap();
        assert_eq!(schema.schema, WIRE_SCHEMA);
        assert_eq!(
            schema.schema_sha256,
            <[u8; 32]>::from(Sha256::digest(WIRE_SCHEMA))
        );
        assert_eq!(schema.release_build_id, fixture.release);
        assert_eq!(schema.wasm_sha256, "01".repeat(32));
        // The fixture's compressed payload is deliberately a different identity.
        assert_ne!(schema.wasm_sha256, "02".repeat(32));

        fs::write(&schema.path, b"changed bytes").unwrap();
        assert!(matches!(
            load_built_wire_schema(&fixture.root, fixture.release, role),
            Err(BuiltDeclarationError::DigestMismatch(_))
        ));
        fs::remove_file(&schema.path).unwrap();
        assert!(matches!(
            load_built_wire_schema(&fixture.root, fixture.release, role),
            Err(BuiltDeclarationError::UnsafeDeclaration(_))
        ));
    }
    let unfinished = Fixture::new(false);
    assert!(matches!(
        load_built_wire_schema(&unfinished.root, unfinished.release, "app"),
        Err(BuiltDeclarationError::ReleaseBuild(_))
    ));
}

#[test]
fn native_registry_binding_requires_release_identity_but_no_candid_sidecar() {
    use crate::protocol_binding::{
        RegistryProtocolBinding, ReleaseProtocolBindingError, resolve_release_registry_wire_schema,
    };
    let fixture = Fixture::new(true);
    let infrastructure =
        load_persisted_canic_infrastructure_artifact_manifest(&fixture.root, fixture.release)
            .unwrap();
    let artifact = infrastructure
        .manifest
        .entries
        .iter()
        .find(|entry| entry.protocol_role.as_str() == "root")
        .unwrap();
    let mut entry = crate::registry::RegistryEntry {
        pid: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        role: Some("root".into()),
        parent_pid: None,
        module_hash: Some(artifact.wasm_sha256_hex.clone()),
        protocol_binding: Some(RegistryProtocolBinding {
            release_identity: artifact.protocol_release_identity.clone(),
            role: artifact.protocol_role.clone(),
            capabilities: artifact.protocol_capabilities.clone(),
            wire_schema_sha256: artifact.wire_schema_sha256,
            protocol_profile_digest: artifact.protocol_profile_digest,
        }),
    };
    fs::remove_file(
        fixture
            .root
            .join(&artifact.wasm_relative_path)
            .with_extension("did"),
    )
    .unwrap();
    let schema =
        resolve_release_registry_wire_schema(&fixture.root, fixture.release, &entry).unwrap();
    assert_eq!(schema.schema, WIRE_SCHEMA);
    entry.module_hash = Some("99".repeat(32));
    assert!(matches!(
        resolve_release_registry_wire_schema(&fixture.root, fixture.release, &entry),
        Err(ReleaseProtocolBindingError::ArtifactBindingMismatch { .. })
    ));
    entry.module_hash = Some(artifact.wasm_sha256_hex.clone());
    entry
        .protocol_binding
        .as_mut()
        .unwrap()
        .release_identity
        .push_str("changed");
    assert!(matches!(
        resolve_release_registry_wire_schema(&fixture.root, fixture.release, &entry),
        Err(ReleaseProtocolBindingError::ArtifactBindingMismatch { .. })
    ));
    entry.protocol_binding.as_mut().unwrap().release_identity =
        artifact.protocol_release_identity.clone();
    fs::write(schema.path, b"unreviewed schema").unwrap();
    assert!(matches!(
        resolve_release_registry_wire_schema(&fixture.root, fixture.release, &entry),
        Err(ReleaseProtocolBindingError::Declaration(
            BuiltDeclarationError::DigestMismatch(_)
        ))
    ));
}
