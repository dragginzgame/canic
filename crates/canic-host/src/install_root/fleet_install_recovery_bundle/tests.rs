use super::*;
use crate::{
    fleet_install_plan::{
        FleetInstallPlan, PlannedCanisterCreationFunding, PlannedFleetCoordinator,
    },
    install_root::fleet_subnet_root_install_journal::{
        begin_root_creation, begin_root_install, begin_store_adoption, begin_store_bootstrap,
        begin_store_staging, begin_wasm_store_creation, begin_wasm_store_install,
        expected_root_authority, expected_wasm_store_authority, record_infrastructure_verified,
        record_root_created, record_root_installed, record_store_adopted,
        record_store_bootstrapped, record_store_staged, record_wasm_store_created,
        record_wasm_store_installed,
    },
    release_build::{
        finalize_release_build_from_manifest, plan_release_build, release_build_plan_path,
    },
};
use candid::Principal;
use canic_core::{
    cdk::utils::hash::sha256_hex,
    dto::{
        fleet_subnet_root::FleetSubnetWasmStoreAdoptionResponse,
        root_store::{RootStoreBootstrapResponse, RootStoreCatalogEntry},
    },
    ids::{
        CanisterRole, CanonicalNetworkId, ComponentTopologyDigest, FleetBinding, FleetId, FleetKey,
        ReleaseBuildId, SubnetId,
    },
    role_contract::ProtocolProfileDigest,
};
use flate2::{Compression, GzBuilder};
use std::io::Write;

use crate::release_set::{
    ApplicationArtifactEntry, ApplicationArtifactUnion, CanicInfrastructureArtifactEntry,
    CanicInfrastructureArtifactManifest, CanicInfrastructureRole,
};

struct FinalizedArtifactFixture {
    files: Vec<(String, Vec<u8>)>,
    application_digest: [u8; 32],
    infrastructure_digest: [u8; 32],
    infrastructure: CanicInfrastructureArtifactManifest,
}

struct ArtifactFixture {
    wasm_relative_path: String,
    wasm: Vec<u8>,
    wasm_gz_relative_path: String,
    wasm_gz: Vec<u8>,
    candid_relative_path: String,
    candid: Vec<u8>,
}

fn finalized_artifact_fixture(release_build_id: ReleaseBuildId) -> FinalizedArtifactFixture {
    let infrastructure_artifacts = [
        (CanicInfrastructureRole::FleetCoordinator, 1_u8),
        (CanicInfrastructureRole::FleetSubnetRoot, 2_u8),
        (CanicInfrastructureRole::WasmStore, 3_u8),
    ]
    .map(|(role, marker)| {
        let artifact = artifact_fixture(release_build_id, role.as_str(), marker);
        let entry = CanicInfrastructureArtifactEntry {
            role,
            package: format!("canic-{}", role.as_str().replace('_', "-")),
            protocol_release_identity: "0.109.9".to_string(),
            protocol_role: CanisterRole::owned(role.protocol_role_name().to_string()),
            protocol_capabilities: std::collections::BTreeSet::new(),
            release_build_id,
            wasm_relative_path: artifact.wasm_relative_path.clone(),
            wasm_size_bytes: artifact.wasm.len() as u64,
            wasm_sha256_hex: sha256_hex(&artifact.wasm),
            wasm_gz_relative_path: artifact.wasm_gz_relative_path.clone(),
            wasm_gz_size_bytes: artifact.wasm_gz.len() as u64,
            wasm_gz_sha256_hex: sha256_hex(&artifact.wasm_gz),
            candid_sha256: Sha256::digest(&artifact.candid).into(),
            protocol_profile_digest: ProtocolProfileDigest::from_bytes([marker; 32]),
        };
        (entry, artifact)
    });
    let infrastructure = CanicInfrastructureArtifactManifest {
        release_build_id,
        entries: infrastructure_artifacts
            .iter()
            .map(|(entry, _)| entry.clone())
            .collect(),
    };
    let infrastructure_bytes = infrastructure
        .canonical_bytes()
        .expect("canonical infrastructure artifact manifest");

    let application_artifact = artifact_fixture(release_build_id, "component", 4);
    let application = ApplicationArtifactUnion {
        release_build_id,
        fleet_component_topology_digest: ComponentTopologyDigest::from_bytes([5; 32]),
        entries: vec![ApplicationArtifactEntry {
            role: CanisterRole::owned("component".to_string()),
            package: "component".to_string(),
            release_build_id,
            wasm_relative_path: application_artifact.wasm_relative_path.clone(),
            wasm_size_bytes: application_artifact.wasm.len() as u64,
            wasm_sha256_hex: sha256_hex(&application_artifact.wasm),
            wasm_gz_relative_path: application_artifact.wasm_gz_relative_path.clone(),
            wasm_gz_size_bytes: application_artifact.wasm_gz.len() as u64,
            wasm_gz_sha256_hex: sha256_hex(&application_artifact.wasm_gz),
            candid_sha256: Sha256::digest(&application_artifact.candid).into(),
            protocol_profile_digest: ProtocolProfileDigest::from_bytes([6; 32]),
        }],
    };
    application
        .validate_retained_shape()
        .expect("canonical application artifact union");
    let application_bytes = serde_json::to_vec(&application).expect("encode application union");

    let release_build_prefix = format!(".canic/release-builds/{release_build_id}");
    let mut files = infrastructure_artifacts
        .into_iter()
        .map(|(_, artifact)| artifact)
        .chain([application_artifact])
        .flat_map(|artifact| {
            [
                (artifact.wasm_relative_path, artifact.wasm),
                (artifact.wasm_gz_relative_path, artifact.wasm_gz),
                (artifact.candid_relative_path, artifact.candid),
            ]
        })
        .collect::<Vec<_>>();
    files.push((
        format!("{release_build_prefix}/{INFRASTRUCTURE_ARTIFACT_MANIFEST_FILE}"),
        infrastructure_bytes.clone(),
    ));
    files.push((
        format!("{release_build_prefix}/{APPLICATION_ARTIFACT_UNION_FILE}"),
        application_bytes.clone(),
    ));

    FinalizedArtifactFixture {
        files,
        application_digest: Sha256::digest(&application_bytes).into(),
        infrastructure_digest: Sha256::digest(&infrastructure_bytes).into(),
        infrastructure,
    }
}

fn artifact_fixture(release_build_id: ReleaseBuildId, role: &str, marker: u8) -> ArtifactFixture {
    let directory = format!(".canic/release-builds/{release_build_id}/artifacts/{role}");
    let mut wasm = vec![0x00, 0x61, 0x73, 0x6d, marker];
    wasm.extend_from_slice(release_build_id.to_string().as_bytes());
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(&wasm).expect("gzip finalized Wasm");
    let wasm_gz = encoder.finish().expect("finish finalized gzip Wasm");
    let candid = format!("service : {{ /* {role} */ }}\n").into_bytes();
    ArtifactFixture {
        wasm_relative_path: format!("{directory}/{role}.wasm"),
        wasm,
        wasm_gz_relative_path: format!("{directory}/{role}.wasm.gz"),
        wasm_gz,
        candid_relative_path: format!("{directory}/{role}.did"),
        candid,
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one fixture freezes the mutually bound session, install plan and finalized release"
)]
fn bundle_fixture() -> (PathBuf, FleetInstallRecoveryBundleV1, Vec<u8>) {
    let root = crate::test_support::temp_dir("fleet-install-recovery-bundle");
    let bundle = root.join("bundle");
    let planned_release = plan_release_build(&root).expect("plan retained release build");
    let release_manifest = root.join("release-set.json");
    write_bytes(&release_manifest, b"retained release set").expect("write release-set manifest");
    let finalized_release = finalize_release_build_from_manifest(
        &root,
        planned_release.record.release_build_id,
        &release_manifest,
    )
    .expect("finalize retained release build");
    let finalized_artifacts = finalized_artifact_fixture(finalized_release.record.release_build_id);
    let session = FleetInstallSession {
        schema_version: 1,
        fleet_name: "primary".parse().expect("Fleet name"),
        fleet: FleetBinding {
            app: "app".into(),
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([0x22; 32]),
            },
        },
        release_build_id: finalized_release.record.release_build_id,
        release_build_plan_digest: [0x34; 32],
        release_set_manifest_digest: [0x35; 32],
        decision_release_build_id: None,
        fresh_fleet_plan_digest: "44".repeat(32),
        operation_id: [0x66; 32],
    };
    let payload =
        encode_canonical_json(&session, CanonicalJsonStyle::Compact, MAX_BUNDLE_FILE_BYTES)
            .expect("encode retained session");
    let fleet_plan = FleetInstallPlan {
        fleet: session.fleet.clone(),
        fresh_fleet_plan_digest: session.fresh_fleet_plan_digest.clone(),
        release_build_id: session.release_build_id,
        application_artifact_union_digest: finalized_artifacts.application_digest,
        admission: crate::test_support::fleet_admission_policy(session.fleet.clone()),
        coordinator: PlannedFleetCoordinator {
            coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[0x37; 29])),
            placement_cost: crate::test_support::placement_cost(SubnetId::from_principal(
                Principal::from_slice(&[0x37; 29]),
            )),
            creation_funding: PlannedCanisterCreationFunding::Cycles {
                cycles: 2_000_000_000_000,
            },
            root_funding: Some(crate::test_support::coordinator_root_funding_policy()),
        },
        fleet_subnet_roots: Vec::new(),
    };
    let plan_payload = encode_canonical_json(
        &fleet_plan,
        CanonicalJsonStyle::Compact,
        MAX_BUNDLE_FILE_BYTES,
    )
    .expect("encode retained Fleet plan");
    let fleet_install_plan_digest: [u8; 32] = Sha256::digest(&plan_payload).into();
    let release_payload = fs::read(release_build_plan_path(&root, session.release_build_id))
        .expect("read retained release-build plan");
    let mut files = vec![
        bundle_file(
            &bundle,
            format!(
                ".canic/recovery/fleet-install-sessions/{}/primary/session.json",
                session.fleet.fleet.canonical_network_id
            ),
            &payload,
        ),
        bundle_file(
            &bundle,
            format!(
                ".canic/recovery/fleet-install-plans/{}/{}/{}/plan.json",
                session.fleet.fleet.canonical_network_id,
                session.fleet.fleet.fleet_id,
                session.release_build_id
            ),
            &plan_payload,
        ),
        bundle_file(
            &bundle,
            format!(
                ".canic/release-builds/{}/plan.cbor",
                session.release_build_id
            ),
            &release_payload,
        ),
    ];
    files.extend(
        finalized_artifacts
            .files
            .into_iter()
            .map(|(logical_path, payload)| bundle_file(&bundle, logical_path, &payload)),
    );
    files.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
    let manifest = FleetInstallRecoveryBundleV1 {
        schema_version: 1,
        canonical_network_id: session.fleet.fleet.canonical_network_id.to_string(),
        fleet_name: session.fleet_name.to_string(),
        fleet_id: session.fleet.fleet.fleet_id.to_string(),
        app_id: session.fleet.app.to_string(),
        release_build_id: session.release_build_id.to_string(),
        fresh_fleet_plan_digest: session.fresh_fleet_plan_digest,
        fleet_install_plan_digest,
        install_operation_id: session.operation_id,
        root_checkpoints: Vec::new(),
        files,
    };
    let manifest_path = bundle.join(MANIFEST_FILE);
    write_bytes(
        &manifest_path,
        &encode_manifest(&manifest_path, &manifest).expect("encode bundle manifest"),
    )
    .expect("write bundle manifest");
    (bundle, manifest, payload)
}

fn bundle_file(
    bundle: &Path,
    logical_path: String,
    payload: &[u8],
) -> FleetInstallRecoveryBundleFileV1 {
    let sha256: [u8; 32] = Sha256::digest(payload).into();
    create_new_bytes_with_parents(&object_path(bundle, sha256), payload)
        .expect("write content-addressed object");
    FleetInstallRecoveryBundleFileV1 {
        logical_path,
        sha256,
        size_bytes: payload.len() as u64,
    }
}

#[test]
fn checkpoint_captures_all_authority_roots_and_excludes_its_own_storage() {
    let (fixture_bundle, manifest, _) = bundle_fixture();
    let root = crate::test_support::temp_dir("fleet-install-bundle-checkpoint");
    for entry in &manifest.files {
        let bytes = fs::read(object_path(&fixture_bundle, entry.sha256))
            .expect("read fixture authority object");
        create_new_bytes_with_parents(&root.join(&entry.logical_path), &bytes)
            .expect("materialize checkpoint source authority");
    }
    let session_entry = manifest
        .files
        .iter()
        .find(|entry| entry.logical_path.ends_with("/session.json"))
        .expect("session entry");
    let session = serde_json::from_slice::<FleetInstallSession>(
        &fs::read(root.join(&session_entry.logical_path)).expect("read source session"),
    )
    .expect("decode source session");
    let plan_entry = manifest
        .files
        .iter()
        .find(|entry| entry.logical_path.ends_with("/plan.json"))
        .expect("plan entry");
    let plan = serde_json::from_slice::<FleetInstallPlan>(
        &fs::read(root.join(&plan_entry.logical_path)).expect("read source plan"),
    )
    .expect("decode source plan");
    let persisted = PersistedFleetInstallPlan {
        plan,
        digest: manifest.fleet_install_plan_digest,
        path: root.join(&plan_entry.logical_path),
        root_release_sets: Vec::new(),
    };
    let fleet_state = root
        .join(".canic/networks")
        .join(&manifest.canonical_network_id)
        .join("fleets")
        .join(&manifest.fleet_id);
    write_bytes(&fleet_state.join("receipts/checkpoint.json"), b"receipt")
        .expect("write Fleet receipt source");
    let checkpoint = fleet_state.join("operator-recovery-bundle");

    checkpoint_bundle_at(&root, &session, &persisted, &checkpoint)
        .expect("checkpoint complete retained authority");
    let report = verify_fleet_install_recovery_bundle(&checkpoint)
        .expect("verify checkpointed retained authority");
    assert_eq!(report.file_count, manifest.files.len() + 1);
    let checkpoint_manifest = serde_json::from_slice::<FleetInstallRecoveryBundleV1>(
        &fs::read(checkpoint.join(MANIFEST_FILE)).expect("read checkpoint manifest"),
    )
    .expect("decode checkpoint manifest");
    assert!(
        checkpoint_manifest
            .files
            .iter()
            .all(|entry| !entry.logical_path.contains("operator-recovery-bundle")),
        "a bundle nested under an admitted source root must not capture itself"
    );
}

#[test]
fn pre_effect_zero_root_bundle_import_survives_deleted_source_workspace() {
    let (bundle, manifest, payload) = bundle_fixture();
    assert!(
        manifest.root_checkpoints.is_empty(),
        "the pre-effect fixture intentionally has no planned Root"
    );
    let report = verify_fleet_install_recovery_bundle(&bundle).expect("verify complete bundle");
    assert_eq!(report.file_count, manifest.files.len());
    assert!(manifest.files.iter().any(|entry| {
        entry.logical_path.ends_with("/component.wasm")
            && entry.logical_path.contains("/artifacts/component/")
    }));
    assert!(manifest.files.iter().any(|entry| {
        entry.logical_path.ends_with("/component.did")
            && entry.logical_path.contains("/artifacts/component/")
    }));

    let imported_root = crate::test_support::temp_dir("fleet-install-bundle-import");
    import_fleet_install_recovery_bundle(&bundle, &imported_root)
        .expect("import complete bundle without remote effects");
    assert_eq!(
        fs::read(
            imported_root.join(
                manifest
                    .files
                    .iter()
                    .find(|entry| entry.logical_path.ends_with("/session.json"))
                    .expect("retained session entry")
                    .logical_path
                    .clone(),
            ),
        )
        .expect("read imported state"),
        payload
    );
    for entry in &manifest.files {
        assert_eq!(
            fs::read(imported_root.join(&entry.logical_path))
                .expect("read imported finalized authority"),
            fs::read(object_path(&bundle, entry.sha256)).expect("read bundled finalized authority")
        );
    }
    import_fleet_install_recovery_bundle(&bundle, &imported_root)
        .expect("exact import replay is idempotent");
}

#[test]
fn bundle_rejects_recomputed_manifest_that_omits_normal_wasm_or_candid() {
    for omitted in ["/component.wasm", "/component.did"] {
        let (bundle, mut manifest, _) = bundle_fixture();
        let original_count = manifest.files.len();
        manifest.files.retain(|entry| {
            !(entry.logical_path.ends_with(omitted)
                && entry.logical_path.contains("/artifacts/component/"))
        });
        assert_eq!(manifest.files.len(), original_count - 1);
        let manifest_path = bundle.join(MANIFEST_FILE);
        write_bytes(
            &manifest_path,
            &encode_manifest(&manifest_path, &manifest).expect("recompute canonical manifest"),
        )
        .expect("publish recomputed incomplete manifest");

        assert!(matches!(
            verify_fleet_install_recovery_bundle(&bundle),
            Err(FleetInstallRecoveryBundleError::InvalidManifest { .. })
        ));
    }
}

#[test]
fn bundle_rejects_tampered_incomplete_mixed_and_unconfined_evidence() {
    let (bundle, manifest, _) = bundle_fixture();
    write_bytes(&object_path(&bundle, manifest.files[0].sha256), b"tampered")
        .expect("tamper bundle object");
    assert!(matches!(
        verify_fleet_install_recovery_bundle(&bundle),
        Err(FleetInstallRecoveryBundleError::DigestMismatch { .. })
    ));

    let (bundle, mut manifest, _) = bundle_fixture();
    manifest.files[0].sha256 = [0x77; 32];
    let manifest_path = bundle.join(MANIFEST_FILE);
    write_bytes(
        &manifest_path,
        &encode_manifest(&manifest_path, &manifest).expect("encode incomplete manifest"),
    )
    .expect("write incomplete manifest");
    assert!(matches!(
        verify_fleet_install_recovery_bundle(&bundle),
        Err(FleetInstallRecoveryBundleError::Missing { .. })
    ));

    let (bundle, mut manifest, _) = bundle_fixture();
    let plan_index = manifest
        .files
        .iter()
        .position(|entry| entry.logical_path.ends_with("/plan.json"))
        .expect("Fleet install plan entry");
    let mut mixed_plan = serde_json::from_slice::<FleetInstallPlan>(
        &fs::read(object_path(&bundle, manifest.files[plan_index].sha256))
            .expect("read retained plan object"),
    )
    .expect("decode retained plan");
    mixed_plan.fresh_fleet_plan_digest = "99".repeat(32);
    let mixed_payload = encode_canonical_json(
        &mixed_plan,
        CanonicalJsonStyle::Compact,
        MAX_BUNDLE_FILE_BYTES,
    )
    .expect("encode mixed plan");
    let mixed_entry = bundle_file(
        &bundle,
        manifest.files[plan_index].logical_path.clone(),
        &mixed_payload,
    );
    manifest.fleet_install_plan_digest = mixed_entry.sha256;
    manifest.files[plan_index] = mixed_entry;
    let manifest_path = bundle.join(MANIFEST_FILE);
    write_bytes(
        &manifest_path,
        &encode_manifest(&manifest_path, &manifest).expect("encode mixed manifest"),
    )
    .expect("write mixed manifest");
    assert!(matches!(
        verify_fleet_install_recovery_bundle(&bundle),
        Err(FleetInstallRecoveryBundleError::InvalidManifest { .. })
    ));

    let (bundle, mut manifest, _) = bundle_fixture();
    manifest.files[0].logical_path = "../foreign/session.json".to_string();
    let manifest_path = bundle.join(MANIFEST_FILE);
    write_bytes(
        &manifest_path,
        &encode_manifest(&manifest_path, &manifest).expect("encode unsafe manifest"),
    )
    .expect("write unsafe manifest");
    assert!(matches!(
        verify_fleet_install_recovery_bundle(&bundle),
        Err(FleetInstallRecoveryBundleError::UnsafePath { .. })
    ));

    let (bundle, manifest, _) = bundle_fixture();
    let imported_root = crate::test_support::temp_dir("fleet-install-bundle-conflict");
    let conflicting = manifest.files.last().expect("last bundle entry");
    write_bytes(&imported_root.join(&conflicting.logical_path), b"foreign")
        .expect("write conflicting destination");
    assert!(matches!(
        import_fleet_install_recovery_bundle(&bundle, &imported_root),
        Err(FleetInstallRecoveryBundleError::ImportConflict { .. })
    ));
    assert!(
        !imported_root.join(&manifest.files[0].logical_path).exists(),
        "a late destination conflict must reject before importing an earlier file"
    );
}

#[test]
fn bundle_rejects_newer_schema_before_import() {
    let (bundle, mut manifest, _) = bundle_fixture();
    manifest.schema_version = 2;
    let manifest_path = bundle.join(MANIFEST_FILE);
    write_bytes(
        &manifest_path,
        &encode_manifest(&manifest_path, &manifest).expect("encode newer manifest"),
    )
    .expect("write newer manifest");
    assert!(matches!(
        verify_fleet_install_recovery_bundle(&bundle),
        Err(FleetInstallRecoveryBundleError::InvalidManifest { .. })
    ));
}

#[test]
fn bundle_completeness_is_derived_from_the_exact_retained_root_phase() {
    let (root, bundle) = retained_root_phase_bundle_fixture();
    let report = verify_fleet_install_recovery_bundle(&bundle)
        .expect("verify phase-complete retained Root bundle");
    assert!(report.file_count > 3);
    let manifest_path = bundle.join(MANIFEST_FILE);
    let mut manifest = serde_json::from_slice::<FleetInstallRecoveryBundleV1>(
        &fs::read(&manifest_path).expect("read phase-complete bundle manifest"),
    )
    .expect("decode phase-complete bundle manifest");
    assert_eq!(manifest.root_checkpoints.len(), 1);
    let journal = manifest.root_checkpoints[0]
        .journal
        .as_ref()
        .expect("retained Root journal checkpoint");
    assert_eq!(journal.sequence, 15);
    assert_eq!(
        journal.phase,
        FleetSubnetRootInstallPhase::StoreBootstrapped
    );
    let journal_entry = manifest
        .files
        .iter()
        .find(|entry| entry.sha256 == journal.sha256)
        .expect("phase-15 Root journal bundle entry")
        .clone();
    fs::remove_dir_all(&root).expect("delete the original retained-install workspace");
    verify_fleet_install_recovery_bundle(&bundle)
        .expect("verify phase-complete bundle after source-workspace loss");
    let imported_root = crate::test_support::temp_dir("fleet-install-phase-bundle-import");
    import_fleet_install_recovery_bundle(&bundle, &imported_root)
        .expect("import the exact phase-15 retained Root authority");
    assert_eq!(
        fs::read(imported_root.join(&journal_entry.logical_path))
            .expect("read imported phase-15 Root journal"),
        fs::read(object_path(&bundle, journal_entry.sha256))
            .expect("read bundled phase-15 Root journal")
    );
    let release_prefix = format!(".canic/release-builds/{}/", manifest.release_build_id);
    let finalized_entries = manifest
        .files
        .iter()
        .filter(|entry| entry.logical_path.starts_with(&release_prefix))
        .collect::<Vec<_>>();
    assert!(finalized_entries.len() > 3);
    for entry in finalized_entries {
        assert_eq!(
            fs::read(imported_root.join(&entry.logical_path))
                .expect("read imported finalized phase-15 artifact"),
            fs::read(object_path(&bundle, entry.sha256))
                .expect("read bundled finalized phase-15 artifact")
        );
    }

    manifest
        .files
        .retain(|entry| !entry.logical_path.ends_with("/root-install-args.bin"));
    write_bytes(
        &manifest_path,
        &encode_manifest(&manifest_path, &manifest).expect("encode incomplete phase manifest"),
    )
    .expect("write incomplete phase manifest");
    assert!(matches!(
        verify_fleet_install_recovery_bundle(&bundle),
        Err(FleetInstallRecoveryBundleError::InvalidManifest { .. })
    ));
    assert!(!root.exists());
}

#[expect(
    clippy::too_many_lines,
    reason = "one fixture establishes the exact phase-15 journal evidence bundle verification consumes"
)]
fn retained_root_phase_bundle_fixture() -> (PathBuf, PathBuf) {
    let root = crate::test_support::temp_dir("fleet-install-phase-bound-bundle");
    let planned_release = plan_release_build(&root).expect("plan retained release build");
    let release_manifest = root.join("release-set.json");
    write_bytes(&release_manifest, b"retained release set").expect("write release-set manifest");
    let finalized_release = finalize_release_build_from_manifest(
        &root,
        planned_release.record.release_build_id,
        &release_manifest,
    )
    .expect("finalize retained release build");
    let finalized_artifacts = finalized_artifact_fixture(finalized_release.record.release_build_id);
    for (logical_path, bytes) in &finalized_artifacts.files {
        write_bytes(&root.join(logical_path), bytes)
            .expect("retain finalized release artifact inventory");
    }
    let session = FleetInstallSession {
        schema_version: 1,
        fleet_name: "primary".parse().expect("Fleet name"),
        fleet: FleetBinding {
            app: "app".into(),
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([0x22; 32]),
            },
        },
        release_build_id: finalized_release.record.release_build_id,
        release_build_plan_digest: [0x34; 32],
        release_set_manifest_digest: [0x35; 32],
        decision_release_build_id: None,
        fresh_fleet_plan_digest: "44".repeat(32),
        operation_id: [0x66; 32],
    };
    let session_path = super::session_path(
        &root,
        session.fleet.fleet.canonical_network_id,
        &session.fleet_name,
    );
    write_bytes(
        &session_path,
        &encode_canonical_json(&session, CanonicalJsonStyle::Compact, MAX_BUNDLE_FILE_BYTES)
            .expect("encode retained session"),
    )
    .expect("retain exact Fleet install session");
    let (mut planned, mut plan) = crate::install_root::fleet_subnet_root_install_journal::tests::planned_repair_fixture_with_root_artifact(
        &root,
        &session,
        Principal::from_slice(&[33]),
        |_| {},
    );
    plan.plan.application_artifact_union_digest = finalized_artifacts.application_digest;
    let plan_bytes =
        serde_json::to_vec(&plan.plan).expect("encode artifact-bound retained Fleet plan");
    plan.digest = Sha256::digest(&plan_bytes).into();
    write_bytes(&plan.path, &plan_bytes).expect("retain artifact-bound Fleet plan");
    planned.journal.fleet_install_plan_digest = plan.digest;
    planned.journal.infrastructure_manifest_digest = finalized_artifacts.infrastructure_digest;
    planned.journal.root_artifact = finalized_artifacts
        .infrastructure
        .entries
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::FleetSubnetRoot)
        .expect("finalized Root artifact")
        .clone();
    planned.journal.expected_root_module_hash =
        artifact_sha256(&planned.journal.root_artifact.wasm_sha256_hex);
    planned.journal.wasm_store_artifact = finalized_artifacts
        .infrastructure
        .entries
        .iter()
        .find(|entry| entry.role == CanicInfrastructureRole::WasmStore)
        .expect("finalized Wasm Store artifact")
        .clone();
    planned.journal.expected_wasm_store_module_hash =
        artifact_sha256(&planned.journal.wasm_store_artifact.wasm_sha256_hex);
    let journal_bytes = encode_canonical_json(
        &planned.journal,
        CanonicalJsonStyle::Compact,
        MAX_BUNDLE_FILE_BYTES,
    )
    .expect("encode artifact-bound Root journal");
    write_bytes(&planned.path, &journal_bytes).expect("retain artifact-bound Root journal");
    let fleet_subnet_root = Principal::from_slice(&[44]);
    let wasm_store = Principal::from_slice(&[55]);
    let controller = Principal::from_slice(&[56]);
    let creating = begin_root_creation(&planned, controller).expect("retain Root creation intent");
    let root_created =
        record_root_created(&creating, fleet_subnet_root).expect("retain created Root");
    let store_creating =
        begin_wasm_store_creation(&root_created).expect("retain Store creation intent");
    let store_created =
        record_wasm_store_created(&store_creating, wasm_store).expect("retain created Store");
    let store_installing =
        begin_wasm_store_install(&store_created).expect("retain Store install intent");
    let store_installed = record_wasm_store_installed(
        &store_installing,
        store_installing.journal.expected_wasm_store_module_hash,
    )
    .expect("retain installed Store");
    let root_installing = begin_root_install(&store_installed).expect("retain Root install intent");
    let root_installed = record_root_installed(
        &root_installing,
        root_installing.journal.expected_root_module_hash,
    )
    .expect("retain installed Root");
    let root_authority =
        expected_root_authority(&root_installed.journal).expect("retained Root authority");
    let store_authority =
        expected_wasm_store_authority(&root_installed.journal).expect("retained Store authority");
    let infrastructure =
        record_infrastructure_verified(&root_installed, root_authority, store_authority.clone())
            .expect("retain verified infrastructure");
    let staging = begin_store_staging(&infrastructure).expect("retain Store staging intent");
    let staged = record_store_staged(&staging).expect("retain staged Store artifacts");
    let adopting = begin_store_adoption(&staged).expect("retain Store adoption intent");
    let mut temporary_controllers = vec![controller, fleet_subnet_root];
    temporary_controllers.sort();
    let adopted = record_store_adopted(
        &adopting,
        FleetSubnetWasmStoreAdoptionResponse {
            operation_id: crate::install_root::root_store_adoption_operation_id(
                session.operation_id,
            ),
            authority: store_authority,
            temporary_controllers,
            final_controllers: vec![fleet_subnet_root],
            adopted_at_ns: 1,
        },
    )
    .expect("retain adopted Store");
    let bootstrapping = begin_store_bootstrap(&adopted).expect("retain Store bootstrap intent");
    let bootstrapped = record_store_bootstrapped(
        &bootstrapping,
        RootStoreBootstrapResponse {
            fleet_subnet_root,
            wasm_store,
            release_set: bootstrapping.journal.root_plan.initial_release_set,
            catalog: vec![RootStoreCatalogEntry {
                role: CanisterRole::from("project_hub"),
                raw_module_hash: [8; 32],
                candid_sha256: [10; 32],
                protocol_profile_digest: ProtocolProfileDigest::from_bytes([11; 32]),
                payload_hash: [9; 32],
                payload_size_bytes: 1_024,
            }],
        },
    )
    .expect("retain sequence-15 Store bootstrap result");
    assert_eq!(bootstrapped.journal.sequence, 15);
    let journal_directory = bootstrapped.path.parent().expect("Root journal directory");
    for (file, bytes) in [
        ("root-create-result.json", b"{}".as_slice()),
        ("wasm-store-create-result.json", b"{}".as_slice()),
        ("wasm-store-install-args.bin", b"store".as_slice()),
        ("root-install-args.bin", b"root".as_slice()),
    ] {
        write_bytes(&journal_directory.join(file), bytes).expect("retain Root phase sidecar");
    }
    let bundle = crate::test_support::temp_dir("fleet-install-phase-bound-bundle-copy");
    checkpoint_bundle_at(&root, &session, &plan, &bundle)
        .expect("checkpoint retained phase-15 bundle");
    (root, bundle)
}

fn artifact_sha256(value: &str) -> [u8; 32] {
    canic_core::cdk::utils::hash::decode_hex(value)
        .expect("canonical artifact SHA-256")
        .try_into()
        .expect("32-byte artifact SHA-256")
}
