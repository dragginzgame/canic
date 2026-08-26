use super::*;
use crate::{
    fleet_install_plan::{
        FleetInstallPlan, PlannedCanisterCreationFunding, PlannedFleetCoordinator,
    },
    release_build::{
        finalize_release_build_from_manifest, plan_release_build, release_build_plan_path,
    },
};
use candid::Principal;
use canic_core::ids::{CanonicalNetworkId, FleetBinding, FleetId, FleetKey, SubnetId};

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
        application_artifact_union_digest: [0x36; 32],
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
    assert_eq!(report.file_count, 4);
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
fn verified_bundle_import_survives_deleted_source_workspace() {
    let (bundle, manifest, payload) = bundle_fixture();
    let report = verify_fleet_install_recovery_bundle(&bundle).expect("verify complete bundle");
    assert_eq!(report.file_count, 3);

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
    import_fleet_install_recovery_bundle(&bundle, &imported_root)
        .expect("exact import replay is idempotent");
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
