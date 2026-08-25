use super::*;
use crate::{
    fleet_catalog::commit_fleet_catalog_entry,
    install_root::{
        fleet_component_provisioning_journal::{
            PlanFleetComponentProvisioningInstallRequest, begin_component_provisioning_preparation,
            begin_fleet_catalog_publication, complete_fleet_component_provisioning_install,
            plan_fleet_component_provisioning_install, record_component_provisioning_observed,
            record_fleet_catalog_published, terminal_component_provisioning_evidence,
        },
        fleet_install_session::{
            CloseFleetInstallSessionRequest, FleetInstallSessionError,
            PlanFleetInstallSessionRequest, close_fleet_install_session,
            plan_fleet_install_session, recover_fleet_install_session_authority,
        },
    },
    release_build::{finalize_release_build_from_manifest, plan_release_build},
};
use canic_core::ids::CanonicalNetworkId;
use pocket_ic::{PocketIc, PocketIcBuilder};
use std::{fs, path::PathBuf, str::FromStr};

#[test]
fn retained_repair_compatibility_is_schema_bounded_not_product_version_bounded() {
    assert_eq!(SUPPORTED_SESSION_SCHEMA_VERSIONS, &[1]);
    assert_eq!(SUPPORTED_ROOT_JOURNAL_SCHEMA_VERSIONS, &[1]);
    assert_eq!(REPAIR_RECEIPT_SCHEMA_VERSION, 1);
}

#[test]
fn receipt_validation_rejects_unsupported_protocol_schemas() {
    let (current, session, receipt) = repair_authority_fixture();
    let mut unsupported_session = session.clone();
    unsupported_session.schema_version = 2;
    assert_invalid(&receipt, &unsupported_session, &current.journal);

    let mut unsupported_journal = current.journal;
    unsupported_journal.schema_version = 2;
    assert_invalid(&receipt, &session, &unsupported_journal);
}

#[test]
fn receipt_validation_rejects_every_changed_retained_authority_binding() {
    let (current, session, mut receipt) = repair_authority_fixture();
    validate_receipt(
        Path::new("receipt.json"),
        &receipt,
        &session,
        &current.journal,
    )
    .expect("exact repair authority");

    for mutate in [
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.schema_version = 2,
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.repair_mode = RetainedRootRepairModeV1::StatePreservingUpgrade;
            receipt.repair_operation_id = [25; 32];
        },
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.session_schema_version = 2,
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.root_journal_schema_version = 2,
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.fleet_name = "other".parse().expect("Fleet name");
        },
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.fleet.app = "other".into(),
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.release_build_id = canic_core::ids::ReleaseBuildId::from_nonce(
                canic_core::ids::ReleaseBuildNonce::from_random_bytes([24; 32]),
            );
        },
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.fleet_install_plan_digest = [31; 32],
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.install_operation_id = [30; 32],
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.fleet_subnet_root = Principal::from_slice(&[43]);
        },
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.installation_controller = Principal::from_slice(&[55]);
        },
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.authority.epoch = 2,
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.fresh_fleet_plan_digest = "cd".repeat(32);
        },
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.infrastructure_manifest_digest = [27; 32];
        },
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.placement_subnet = SubnetId::from_principal(Principal::from_slice(&[42; 29]));
        },
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.retained_journal_sequence += 1,
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.predecessor_module_sha256 = [26; 32];
        },
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.predecessor_candid_sha256 = [23; 32];
        },
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.successor_module_sha256 = [28; 32];
        },
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.successor_candid_sha256 = [29; 32],
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.successor_wasm_size_bytes = MAX_REPAIR_WASM_BYTES as u64 + 1;
        },
    ] {
        let mut changed = receipt.clone();
        mutate(&mut changed);
        assert_invalid(&changed, &session, &current.journal);
    }

    receipt.successor_module_sha256 = receipt.predecessor_module_sha256;
    assert_invalid(&receipt, &session, &current.journal);
}

#[test]
fn repair_receipt_publication_is_immutable_and_exactly_replayable() {
    let (current, session, receipt) = repair_authority_fixture();
    let resolved = ResolvedRetainedRootRepair {
        receipt: receipt.clone(),
        needs_publication: true,
        path: repair_receipt_path(&current.path),
    };
    publish_retained_root_repair(&resolved, &session, &current.journal)
        .expect("publish exact repair receipt");
    let replay = resolve_retained_root_repair(&current, &session, None)
        .expect("reload exact repair receipt")
        .expect("retained repair receipt");
    assert_eq!(replay.receipt, receipt);
    assert!(!replay.needs_publication);
}

#[test]
#[ignore = "the workspace runner supplies the governed serial PocketIC server"]
fn retained_repair_adoption_reaches_component_catalog_completion_and_closes_recovery() {
    let fixture = retained_repair_journey_fixture();
    let receipt = adopt_exact_repair_and_reject_conflicts(&fixture);
    assert_unrelated_live_module_rejects(&fixture, &receipt);
    install_qualification_component(&fixture);
    complete_catalog_and_close_recovery(&fixture);
}

struct RetainedRepairJourneyFixture {
    root: PathBuf,
    pic: PocketIc,
    controller: Principal,
    fleet_subnet_root: Principal,
    current: ResolvedFleetSubnetRootInstall,
    session: FleetInstallSession,
    successor_path: PathBuf,
    changed_path: PathBuf,
    wrong_candid_path: PathBuf,
    successor_module_sha256: [u8; 32],
}

struct RetainedRepairArtifacts {
    predecessor_wasm: Vec<u8>,
    successor_wasm: Vec<u8>,
    predecessor_path: PathBuf,
    successor_path: PathBuf,
    changed_path: PathBuf,
    wrong_candid_path: PathBuf,
}

fn retained_repair_journey_fixture() -> RetainedRepairJourneyFixture {
    let root = crate::test_support::temp_dir("retained-root-repair-pocketic-completion");
    fs::create_dir_all(&root).expect("create retained-repair qualification root");
    let artifacts = write_retained_repair_artifacts(&root);
    let session = plan_qualification_session(&root);
    let (pic, controller, fleet_subnet_root, successor_module_sha256) =
        install_repaired_root(&artifacts);
    let mut current =
        crate::install_root::fleet_subnet_root_install_journal::tests::planned_repair_fixture(
            &root,
        );
    current.journal.phase = FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified;
    current.journal.sequence = 28;
    current.journal.fleet_subnet_root = Some(fleet_subnet_root);
    current.journal.installation_controller = Some(controller);
    current.journal.release_build_id = session.release_build_id;
    current.journal.install_operation_id = session.operation_id;
    current.journal.authority.binding.fleet = session.fleet.clone();
    current.journal.expected_root_module_hash = Sha256::digest(&artifacts.predecessor_wasm).into();
    current.journal.root_artifact.candid_sha256 = Sha256::digest(
        extract_candid_bytes(&artifacts.predecessor_path).expect("predecessor Candid"),
    )
    .into();

    RetainedRepairJourneyFixture {
        root,
        pic,
        controller,
        fleet_subnet_root,
        current,
        session,
        successor_path: artifacts.successor_path,
        changed_path: artifacts.changed_path,
        wrong_candid_path: artifacts.wrong_candid_path,
        successor_module_sha256,
    }
}

fn write_retained_repair_artifacts(root: &Path) -> RetainedRepairArtifacts {
    let predecessor_wasm = minimal_canister_wasm("service : {}\n", "predecessor");
    let successor_wasm = minimal_canister_wasm("service : {}\n", "successor");
    let changed_wasm = minimal_canister_wasm("service : {}\n", "changed-successor");
    let wrong_candid_wasm =
        minimal_canister_wasm("service : { ping: () -> (); }\n", "wrong-candid");
    let predecessor_path = root.join("predecessor-root.wasm");
    let successor_path = root.join("successor-root.wasm");
    let changed_path = root.join("changed-root.wasm");
    let wrong_candid_path = root.join("wrong-candid-root.wasm");
    fs::write(&predecessor_path, &predecessor_wasm).expect("write predecessor Wasm");
    fs::write(&successor_path, &successor_wasm).expect("write successor Wasm");
    fs::write(&changed_path, &changed_wasm).expect("write changed successor Wasm");
    fs::write(&wrong_candid_path, &wrong_candid_wasm).expect("write wrong-Candid Wasm");

    RetainedRepairArtifacts {
        predecessor_wasm,
        successor_wasm,
        predecessor_path,
        successor_path,
        changed_path,
        wrong_candid_path,
    }
}

fn plan_qualification_session(root: &Path) -> FleetInstallSession {
    let finalized_plan = plan_release_build(root).expect("plan retained release build");
    let manifest = root.join("release-set.json");
    fs::write(&manifest, [7; 32]).expect("write release-set authority");
    let finalized = finalize_release_build_from_manifest(
        root,
        finalized_plan.record.release_build_id,
        &manifest,
    )
    .expect("finalize retained release build");
    plan_fleet_install_session(PlanFleetInstallSessionRequest {
        root,
        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
        fleet_name: "main".parse().expect("Fleet name"),
        app: "demo".into(),
        finalized_release_build: &finalized,
        decision_release_build_id: None,
        fresh_fleet_plan_digest: "abababababababababababababababababababababababababababababababab",
    })
    .expect("publish retained install session")
}

fn install_repaired_root(
    artifacts: &RetainedRepairArtifacts,
) -> (PocketIc, Principal, Principal, [u8; 32]) {
    let mut pic_builder = PocketIcBuilder::new().with_application_subnet();
    if let Ok(server_url) = std::env::var("CANIC_POCKET_IC_SERVER_URL") {
        pic_builder =
            pic_builder.with_server_url(server_url.parse().expect("governed PocketIC server URL"));
    }
    let pic = pic_builder.build();
    let controller = Principal::from_slice(&[56]);
    let fleet_subnet_root = pic.create_canister_with_settings(Some(controller), None);
    pic.install_canister(
        fleet_subnet_root,
        artifacts.predecessor_wasm.clone(),
        Vec::new(),
        Some(controller),
    );
    pic.upgrade_canister(
        fleet_subnet_root,
        artifacts.successor_wasm.clone(),
        Vec::new(),
        Some(controller),
    )
    .expect("apply the exact state-preserving Root repair");
    let live_root = pic
        .canister_status(fleet_subnet_root, Some(controller))
        .expect("read repaired Root status");
    let successor_module_sha256: [u8; 32] = Sha256::digest(&artifacts.successor_wasm).into();
    assert_eq!(
        live_root.module_hash,
        Some(successor_module_sha256.to_vec())
    );
    assert_eq!(live_root.settings.controllers, vec![controller]);
    (pic, controller, fleet_subnet_root, successor_module_sha256)
}

fn adopt_exact_repair_and_reject_conflicts(
    fixture: &RetainedRepairJourneyFixture,
) -> RetainedRootRepairReceiptV1 {
    let wrong_candid = repair_adoption(fixture.fleet_subnet_root, &fixture.wrong_candid_path);
    assert!(matches!(
        resolve_retained_root_repair(&fixture.current, &fixture.session, Some(&wrong_candid)),
        Err(RetainedRootRepairError::CandidMismatch)
    ));

    let adoption = repair_adoption(fixture.fleet_subnet_root, &fixture.successor_path);
    let repair = resolve_retained_root_repair(&fixture.current, &fixture.session, Some(&adoption))
        .expect("compile exact repair adoption")
        .expect("repair candidate");
    assert_eq!(
        repair.receipt.successor_module_hash(),
        fixture.successor_module_sha256
    );
    publish_retained_root_repair(&repair, &fixture.session, &fixture.current.journal)
        .expect("publish exact repair receipt after live PocketIC verification");

    let changed = repair_adoption(fixture.fleet_subnet_root, &fixture.changed_path);
    assert!(matches!(
        resolve_retained_root_repair(&fixture.current, &fixture.session, Some(&changed)),
        Err(RetainedRootRepairError::ConflictingAuthority { .. })
    ));
    let mut wrong_authority = fixture.session.clone();
    wrong_authority.operation_id[0] ^= 1;
    assert!(matches!(
        resolve_retained_root_repair(&fixture.current, &wrong_authority, None),
        Err(RetainedRootRepairError::InvalidDocument { .. })
    ));
    repair.receipt
}

fn assert_unrelated_live_module_rejects(
    fixture: &RetainedRepairJourneyFixture,
    receipt: &RetainedRootRepairReceiptV1,
) {
    let unrelated_wasm = minimal_canister_wasm("service : {}\n", "unrelated-root");
    let unrelated_root = fixture
        .pic
        .create_canister_with_settings(Some(fixture.controller), None);
    fixture.pic.install_canister(
        unrelated_root,
        unrelated_wasm,
        Vec::new(),
        Some(fixture.controller),
    );
    let unrelated_hash = fixture
        .pic
        .canister_status(unrelated_root, Some(fixture.controller))
        .expect("read unrelated Root status")
        .module_hash
        .expect("unrelated Root module hash");
    assert_ne!(unrelated_hash, receipt.successor_module_hash());
}

fn install_qualification_component(fixture: &RetainedRepairJourneyFixture) {
    let component = fixture
        .pic
        .create_canister_with_settings(Some(fixture.controller), None);
    let component_wasm = minimal_canister_wasm("service : {}\n", "component");
    fixture.pic.install_canister(
        component,
        component_wasm.clone(),
        Vec::new(),
        Some(fixture.controller),
    );
    assert_eq!(
        fixture
            .pic
            .canister_status(component, Some(fixture.controller))
            .expect("read installed Component status")
            .module_hash,
        Some(Sha256::digest(component_wasm).to_vec())
    );
}

fn complete_catalog_and_close_recovery(fixture: &RetainedRepairJourneyFixture) {
    let mut component_plan =
        crate::install_root::fleet_component_provisioning_journal::tests::install_plan(
            &fixture.root,
        );
    component_plan.plan.fleet = fixture.session.fleet.clone();
    component_plan.plan.release_build_id = fixture.session.release_build_id;
    component_plan.plan.fresh_fleet_plan_digest = fixture.session.fresh_fleet_plan_digest.clone();
    let compiled = crate::install_root::fleet_component_provisioning_journal::tests::compiled_plan(
        &component_plan,
    );
    let planned =
        plan_fleet_component_provisioning_install(PlanFleetComponentProvisioningInstallRequest {
            fleet_install_plan: &component_plan,
            coordinator: Principal::from_slice(&[3; 29]),
            fleet_name: fixture.session.fleet_name.clone(),
            environment: "ic".to_string(),
            compiled: compiled.clone(),
        })
        .expect("plan retained Component completion");
    let preparing =
        begin_component_provisioning_preparation(&planned).expect("persist preparation intent");
    let runtime = record_component_provisioning_observed(
        &preparing,
        crate::install_root::fleet_component_provisioning_journal::tests::terminal_status(
            &compiled,
        ),
    )
    .expect("retain direct terminal runtime evidence");
    let entry = crate::install_root::fleet_component_provisioning_journal::tests::catalog_entry(
        &component_plan,
        100,
    );
    let publishing = begin_fleet_catalog_publication(&runtime, entry.clone())
        .expect("retain exact catalog intent");
    let committed =
        commit_fleet_catalog_entry(&fixture.root, entry).expect("publish Fleet catalog");
    assert!(committed.advanced);
    let published = record_fleet_catalog_published(&publishing, committed)
        .expect("retain Fleet catalog receipt");
    let complete = complete_fleet_component_provisioning_install(&published)
        .expect("complete Component provisioning journal");
    let terminal = terminal_component_provisioning_evidence(&complete)
        .expect("bind exact terminal Component journal");
    close_fleet_install_session(CloseFleetInstallSessionRequest {
        root: &fixture.root,
        session: &fixture.session,
        component_journal: &terminal,
    })
    .expect("publish terminal completion receipt");

    assert!(matches!(
        recover_fleet_install_session_authority(
            &fixture.root,
            fixture.session.fleet.fleet.canonical_network_id,
            &fixture.session.fleet_name,
            &fixture.session.fleet.app,
        ),
        Err(FleetInstallSessionError::Completed { .. })
    ));
}

fn repair_adoption(root: Principal, wasm: &Path) -> RetainedRootRepairAdoption {
    RetainedRootRepairAdoption::from_str(&format!("{root}={}", wasm.display()))
        .expect("parse --adopt-retained-root-repair value")
}

fn repair_authority_fixture() -> (
    ResolvedFleetSubnetRootInstall,
    FleetInstallSession,
    RetainedRootRepairReceiptV1,
) {
    let root = crate::test_support::temp_dir("retained-root-repair-authority");
    let mut current =
        crate::install_root::fleet_subnet_root_install_journal::tests::planned_repair_fixture(
            &root,
        );
    let root_canister = Principal::from_slice(&[44]);
    let controller = Principal::from_slice(&[56]);
    current.journal.phase = FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified;
    current.journal.sequence = 28;
    current.journal.fleet_subnet_root = Some(root_canister);
    current.journal.installation_controller = Some(controller);
    let session = FleetInstallSession {
        schema_version: 1,
        fleet_name: "primary".parse().expect("Fleet name"),
        fleet: current.journal.authority.binding.fleet.clone(),
        release_build_id: current.journal.release_build_id,
        release_build_plan_digest: [12; 32],
        release_set_manifest_digest: current.journal.infrastructure_manifest_digest,
        decision_release_build_id: Some(current.journal.release_build_id),
        fresh_fleet_plan_digest: "ab".repeat(32),
        operation_id: current.journal.install_operation_id,
    };
    let receipt = fixture_receipt(&session, &current.journal);
    (current, session, receipt)
}

fn assert_invalid(
    receipt: &RetainedRootRepairReceiptV1,
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
) {
    assert!(matches!(
        validate_receipt(Path::new("receipt.json"), receipt, session, journal,),
        Err(RetainedRootRepairError::InvalidDocument { .. })
    ));
}

fn fixture_receipt(
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
) -> RetainedRootRepairReceiptV1 {
    let fleet_subnet_root = journal.fleet_subnet_root.expect("Root");
    let successor_module_sha256 = [8; 32];
    RetainedRootRepairReceiptV1 {
        schema_version: 1,
        repair_operation_id: repair_operation_id(
            session,
            journal,
            successor_module_sha256,
            1,
            journal.root_artifact.candid_sha256,
        )
        .expect("repair operation identity"),
        repair_mode: RetainedRootRepairModeV1::StatePreservingUpgrade,
        session_schema_version: session.schema_version,
        root_journal_schema_version: journal.schema_version,
        fleet_name: session.fleet_name.clone(),
        fleet: session.fleet.clone(),
        release_build_id: session.release_build_id,
        fresh_fleet_plan_digest: session.fresh_fleet_plan_digest.clone(),
        fleet_install_plan_digest: journal.fleet_install_plan_digest,
        infrastructure_manifest_digest: journal.infrastructure_manifest_digest,
        install_operation_id: session.operation_id,
        authority: journal.authority.clone(),
        placement_subnet: journal.root_plan.placement_subnet,
        fleet_subnet_root,
        installation_controller: journal.installation_controller.expect("controller"),
        retained_journal_phase: journal.phase,
        retained_journal_sequence: journal.sequence,
        predecessor_module_sha256: journal.expected_root_module_hash,
        successor_module_sha256,
        successor_wasm_size_bytes: 1,
        predecessor_candid_sha256: journal.root_artifact.candid_sha256,
        successor_candid_sha256: journal.root_artifact.candid_sha256,
    }
}

fn minimal_canister_wasm(candid: &str, marker: &str) -> Vec<u8> {
    let mut wasm = b"\0asm\x01\0\0\0".to_vec();
    // Export one memory plus `get_candid_pointer`, the exact production
    // candid-extractor contract, without adding application behavior.
    append_wasm_section(&mut wasm, 1, &[1, 0x60, 0, 1, 0x7f]);
    append_wasm_section(&mut wasm, 3, &[1, 0]);
    append_wasm_section(&mut wasm, 5, &[1, 0, 1]);
    let mut exports = vec![2, 6];
    exports.extend_from_slice(b"memory");
    exports.extend_from_slice(&[2, 0, 18]);
    exports.extend_from_slice(b"get_candid_pointer");
    exports.extend_from_slice(&[0, 0]);
    append_wasm_section(&mut wasm, 7, &exports);
    append_wasm_section(&mut wasm, 10, &[1, 4, 0, 0x41, 0, 0x0b]);
    let mut data = vec![1, 0, 0x41, 0, 0x0b];
    push_unsigned_leb128(&mut data, candid.len() + 1);
    data.extend_from_slice(candid.as_bytes());
    data.push(0);
    append_wasm_section(&mut wasm, 11, &data);
    append_custom_section(&mut wasm, "canic:retained-repair-test", marker.as_bytes());
    wasm
}

fn append_wasm_section(wasm: &mut Vec<u8>, section_id: u8, payload: &[u8]) {
    wasm.push(section_id);
    push_unsigned_leb128(wasm, payload.len());
    wasm.extend_from_slice(payload);
}

fn append_custom_section(wasm: &mut Vec<u8>, name: &str, value: &[u8]) {
    let mut payload = Vec::new();
    push_unsigned_leb128(&mut payload, name.len());
    payload.extend_from_slice(name.as_bytes());
    payload.extend_from_slice(value);
    wasm.push(0);
    push_unsigned_leb128(wasm, payload.len());
    wasm.extend_from_slice(&payload);
}

fn push_unsigned_leb128(bytes: &mut Vec<u8>, mut value: usize) {
    loop {
        let mut byte = u8::try_from(value & 0x7f).expect("seven-bit LEB128 lane");
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        bytes.push(byte);
        if value == 0 {
            return;
        }
    }
}
