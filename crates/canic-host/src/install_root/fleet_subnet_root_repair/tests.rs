use super::*;
use crate::{
    fleet_catalog::commit_fleet_catalog_entry,
    icp::LocalReplicaTarget,
    install_root::{
        fleet_component_provisioning_journal::{
            PlanFleetComponentProvisioningInstallRequest, begin_component_provisioning_preparation,
            begin_fleet_catalog_publication, complete_fleet_component_provisioning_install,
            plan_fleet_component_provisioning_install, record_component_provisioning_observed,
            record_fleet_catalog_published, terminal_component_provisioning_evidence,
        },
        fleet_install_recovery_bundle::FleetInstallRecoveryBundleCheckpoint,
        fleet_install_session::{
            CloseFleetInstallSessionRequest, FleetInstallSessionError,
            PlanFleetInstallSessionRequest, close_fleet_install_session,
            plan_fleet_install_session, recover_fleet_install_session_authority,
        },
        fleet_subnet_root_component_registry_preparation::verify_retained_component_registry_preparation,
        fleet_subnet_root_install::verify_pre_repair_root_authority,
        fleet_subnet_root_install_journal::{
            FleetSubnetRootInstallJournal, begin_root_creation, begin_root_install,
            begin_store_adoption, begin_store_bootstrap, begin_store_staging,
            begin_wasm_store_creation, begin_wasm_store_install, expected_registry_join_entry,
            expected_root_authority, expected_wasm_store_authority, record_infrastructure_verified,
            record_root_created, record_root_installed, record_store_adopted,
            record_store_bootstrapped, record_store_staged, record_wasm_store_created,
            record_wasm_store_installed,
        },
        icp_context::InstallIcpContext,
        operations::require_expected_module_hash,
    },
    protocol_binding::resolve_infrastructure_protocol_binding,
    release_build::{finalize_release_build_from_manifest, plan_release_build},
    release_set::{
        APPLICATION_ARTIFACT_UNION_FILE, ApplicationArtifactEntry, ApplicationArtifactUnion,
        CanicInfrastructureArtifactEntry, CanicInfrastructureArtifactManifest,
        CanicInfrastructureRole, INFRASTRUCTURE_ARTIFACT_MANIFEST_FILE,
    },
};
use candid::CandidType;
use canic_control_plane::dto::root::RootRegistrySynchronizationOperationStatus;
use canic_core::{
    control_plane_support::ops::fleet_registry::FleetRegistryOps,
    dto::{
        component_registry::{
            RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
        },
        fleet_registry::{
            FleetRegistry, FleetRegistryManifest, FleetRegistryVersion,
            FleetSubnetRootJoinResponse, FleetSubnetRootRegistryMirrorActivationRequest,
            FleetSubnetRootRegistryMirrorActivationResponse, FleetSubnetRootRegistrySyncRequest,
            FleetSubnetRootRegistrySyncResponse, FleetSubnetRootSnapshotAcknowledgement,
        },
        fleet_subnet_root::{FleetSubnetRootAuthority, FleetSubnetWasmStoreAdoptionResponse},
        root_store::{
            RootStoreBootstrapRequest, RootStoreBootstrapResponse, RootStoreCatalogEntry,
        },
    },
    ids::{CanisterRole, CanonicalNetworkId, FleetSubnetRootReleaseSet},
    role_contract::{ProtocolProfileDigest, RoleCapabilityKey, derive_protocol_profile_hashes},
};
use flate2::{Compression, GzBuilder};
use pocket_ic::common::rest::{IcpFeatures, IcpFeaturesConfig};
use pocket_ic::{CreateCanisterParams, CreateCanisterPlacement, PocketIc, PocketIcBuilder};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
    time::SystemTime,
};

#[test]
fn retained_repair_compatibility_is_schema_bounded_not_product_version_bounded() {
    assert_eq!(SUPPORTED_SESSION_SCHEMA_VERSIONS, &[1]);
    assert_eq!(SUPPORTED_ROOT_JOURNAL_SCHEMA_VERSIONS, &[1]);
    assert_eq!(REPAIR_RECEIPT_SCHEMA_VERSION, 1);
}

#[test]
fn repair_compilation_binds_an_already_upgraded_predecessor_and_exact_candid_successor() {
    let (mut current, session, _) = repair_authority_fixture();
    let artifacts =
        write_retained_repair_artifacts(current.path.parent().expect("repair journal directory"));
    current.journal.expected_root_module_hash = Sha256::digest(&artifacts.predecessor_wasm).into();
    current.journal.root_artifact.wasm_size_bytes =
        u64::try_from(artifacts.predecessor_wasm.len()).expect("retained Wasm size");
    current.journal.root_artifact.candid_sha256 = Sha256::digest(
        fs::read(artifacts.predecessor_path.with_extension("did"))
            .expect("retained Candid sidecar"),
    )
    .into();
    assert!(!wasm_exports_candid_pointer(&artifacts.predecessor_wasm));
    let adoption = repair_adoption(
        current.journal.fleet_subnet_root.expect("Root"),
        &artifacts.live_predecessor_path,
        &artifacts.successor_path,
    );

    let receipt = compile_adoption(&session, &current.journal, &adoption, 5_000_000_000_000)
        .expect("compile exact-Candid two-artifact repair");
    let live_predecessor_hash: [u8; 32] = Sha256::digest(&artifacts.live_predecessor_wasm).into();
    let successor_hash: [u8; 32] = Sha256::digest(&artifacts.successor_wasm).into();
    assert_eq!(
        receipt.upgrade_predecessor_module_sha256,
        live_predecessor_hash
    );
    assert_eq!(receipt.successor_module_sha256, successor_hash);
    assert_eq!(
        receipt.successor_candid_sha256,
        receipt.retained_journal_candid_sha256
    );

    let wrong = repair_adoption(
        current.journal.fleet_subnet_root.expect("Root"),
        &artifacts.live_predecessor_path,
        &artifacts.wrong_candid_path,
    );
    assert!(matches!(
        compile_adoption(&session, &current.journal, &wrong, 5_000_000_000_000),
        Err(RetainedRootRepairError::CandidMismatch(_))
    ));
}

#[test]
fn repair_candid_resolution_requires_bounded_exact_sidecars_and_limits_extraction_to_build_exports()
{
    let root = crate::test_support::temp_dir("retained-root-repair-candid-resolution");
    fs::create_dir_all(&root).expect("create Candid resolution fixture root");
    let artifacts = write_retained_repair_artifacts(&root);
    let predecessor_sidecar = artifacts.predecessor_path.with_extension("did");
    fs::remove_file(&predecessor_sidecar).expect("remove finalized predecessor sidecar");
    assert!(matches!(
        inspect_wasm(
            &artifacts.predecessor_path,
            RepairCandidResolution::FinalizedSidecar,
        ),
        Err(RetainedRootRepairError::FinalizedCandidSidecarMissing { .. })
    ));

    let successor_sidecar = artifacts.successor_path.with_extension("did");
    fs::remove_file(&successor_sidecar).expect("remove successor sidecar");
    assert!(matches!(
        inspect_wasm(
            &artifacts.successor_path,
            RepairCandidResolution::SuccessorSidecarOrBuildExport,
        ),
        Err(RetainedRootRepairError::SuccessorCandidUnavailable { .. })
    ));

    let build_output = root.join("successor-build-output.wasm");
    fs::write(
        &build_output,
        minimal_canister_wasm("service : { ping: () -> (); }\n", "build-output"),
    )
    .expect("write successor build output");
    let extracted = inspect_wasm(
        &build_output,
        RepairCandidResolution::SuccessorSidecarOrBuildExport,
    )
    .expect("use candid-extractor only for a build output with the debug export");
    assert_eq!(
        std::str::from_utf8(&extracted.candid)
            .expect("extracted Candid is UTF-8")
            .trim_end(),
        "service : { ping: () -> (); }"
    );

    let oversized = root.join("oversized-root.wasm");
    fs::write(&oversized, &artifacts.predecessor_wasm).expect("write oversized-sidecar Wasm");
    fs::write(
        oversized.with_extension("did"),
        vec![b' '; MAX_REPAIR_CANDID_BYTES + 1],
    )
    .expect("write oversized sidecar");
    assert!(matches!(
        inspect_wasm(&oversized, RepairCandidResolution::FinalizedSidecar),
        Err(RetainedRootRepairError::CandidSidecarTooLarge { .. })
    ));
}

#[cfg(unix)]
#[test]
fn repair_candid_resolution_rejects_unsafe_and_invalid_sidecars() {
    use std::os::unix::fs::symlink;

    let root = crate::test_support::temp_dir("retained-root-repair-candid-safety");
    fs::create_dir_all(&root).expect("create Candid safety fixture root");
    let artifacts = write_retained_repair_artifacts(&root);
    let successor_sidecar = artifacts.successor_path.with_extension("did");
    fs::remove_file(&successor_sidecar).expect("remove regular successor sidecar");
    symlink(
        artifacts.predecessor_path.with_extension("did"),
        &successor_sidecar,
    )
    .expect("link unsafe successor sidecar");
    assert!(matches!(
        inspect_wasm(
            &artifacts.successor_path,
            RepairCandidResolution::SuccessorSidecarOrBuildExport,
        ),
        Err(RetainedRootRepairError::CandidSidecarUnsafe { .. })
    ));

    fs::remove_file(&successor_sidecar).expect("remove unsafe sidecar");
    fs::write(&successor_sidecar, [0xff]).expect("write invalid sidecar");
    assert!(matches!(
        inspect_wasm(
            &artifacts.successor_path,
            RepairCandidResolution::SuccessorSidecarOrBuildExport,
        ),
        Err(RetainedRootRepairError::InvalidCandidSidecar { .. })
    ));
}

#[test]
fn provisional_authority_is_phase_bounded_and_survives_disposable_artifact_deletion() {
    let (mut current, session, _) = repair_authority_fixture();
    current.journal.phase = FleetSubnetRootInstallPhase::StoreBootstrapped;
    current.journal.sequence = 15;
    current.journal.component_registry_preparation_request = None;
    current.journal.component_registry_preparation_response = None;
    let artifacts =
        write_retained_repair_artifacts(current.path.parent().expect("repair journal directory"));
    current.journal.expected_root_module_hash = Sha256::digest(&artifacts.predecessor_wasm).into();
    current.journal.root_artifact.wasm_size_bytes =
        u64::try_from(artifacts.predecessor_wasm.len()).expect("retained Wasm size");
    current.journal.root_artifact.candid_sha256 = Sha256::digest(
        fs::read(artifacts.predecessor_path.with_extension("did"))
            .expect("retained Candid sidecar"),
    )
    .into();
    let adoption = repair_adoption(
        current.journal.fleet_subnet_root.expect("Root"),
        &artifacts.live_predecessor_path,
        &artifacts.successor_path,
    );
    let repair =
        resolve_retained_root_repair(&current, &session, Some(&adoption), Some(5_000_000_000_000))
            .expect("compile sequence-15 provisional authority")
            .expect("repair authority");
    assert_eq!(
        repair.authority.authority_journal_phase,
        FleetSubnetRootInstallPhase::StoreBootstrapped
    );
    assert!(matches!(
        publish_retained_root_repair_receipt(&repair, &session, &current.journal),
        Err(RetainedRootRepairError::PrematureTerminalReceipt)
    ));
    fs::remove_file(&artifacts.live_predecessor_path).expect("delete caller predecessor artifact");
    fs::remove_file(artifacts.live_predecessor_path.with_extension("did"))
        .expect("delete caller predecessor Candid sidecar");
    fs::remove_file(&artifacts.successor_path).expect("delete caller successor artifact");
    fs::remove_file(artifacts.successor_path.with_extension("did"))
        .expect("delete caller successor Candid sidecar");
    let candidate_replay =
        resolve_retained_root_repair(&current, &session, None, Some(5_000_000_000_000))
            .expect("reload candidate without caller-owned artifact paths")
            .expect("retained repair candidate");
    assert!(candidate_replay.needs_authority_publication);
    publish_retained_root_repair_authority(&candidate_replay, &session, &current.journal)
        .expect("publish exact provisional authority after candidate replay");
    let replay = resolve_retained_root_repair(&current, &session, None, Some(5_000_000_000_000))
        .expect("reload published authority without caller-owned artifact paths")
        .expect("retained provisional authority");
    assert!(!replay.needs_authority_publication);
    assert!(replay.successor_wasm_path.is_file());

    let mut pre_infrastructure = current.journal;
    pre_infrastructure.phase = FleetSubnetRootInstallPhase::InfrastructureVerified;
    pre_infrastructure.sequence = 9;
    assert!(matches!(
        validate_authority(
            Path::new("authority.json"),
            &repair.authority,
            &session,
            &pre_infrastructure,
            None,
        ),
        Err(RetainedRootRepairError::InvalidPhase)
    ));
}

#[test]
fn provisional_authority_remains_valid_after_canonical_journal_advancement() {
    let (mut current, session, authority) = repair_authority_fixture();
    let mut origin = authority;
    origin.authority_journal_phase = FleetSubnetRootInstallPhase::StoreBootstrapped;
    origin.authority_journal_sequence = 15;
    origin.repair_operation_id = repair_operation_id(
        &session,
        &current.journal,
        &RetainedRootRepairTransition::from_authority(&origin),
        origin.authority_journal_phase,
        origin.authority_journal_sequence,
    )
    .expect("compile origin-bound operation identity");
    validate_authority(
        Path::new("authority.json"),
        &origin,
        &session,
        &current.journal,
        None,
    )
    .expect("sequence-28 journal accepts exact sequence-15 authority");

    current.journal.sequence = 14;
    assert!(matches!(
        validate_authority(
            Path::new("authority.json"),
            &origin,
            &session,
            &current.journal,
            None,
        ),
        Err(RetainedRootRepairError::InvalidDocument { .. })
    ));
}

#[test]
fn provisional_authority_window_names_protocol_phases_not_sequences() {
    let accepted = [
        FleetSubnetRootInstallPhase::StoreBootstrapped,
        FleetSubnetRootInstallPhase::StoreVerified,
        FleetSubnetRootInstallPhase::RegistryJoinInFlight,
        FleetSubnetRootInstallPhase::RegistryJoined,
        FleetSubnetRootInstallPhase::RegistryJoinVerified,
        FleetSubnetRootInstallPhase::RegistrySyncInFlight,
        FleetSubnetRootInstallPhase::RegistrySynchronized,
        FleetSubnetRootInstallPhase::RegistrySyncVerified,
        FleetSubnetRootInstallPhase::RegistryMirrorActivationInFlight,
        FleetSubnetRootInstallPhase::RegistryMirrorActivated,
        FleetSubnetRootInstallPhase::RegistryMirrorActivationVerified,
        FleetSubnetRootInstallPhase::ComponentRegistryPreparationInFlight,
        FleetSubnetRootInstallPhase::ComponentRegistryPrepared,
        FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified,
    ];
    assert!(
        accepted
            .into_iter()
            .all(FleetSubnetRootInstallPhase::admits_retained_root_repair)
    );
    assert!(!FleetSubnetRootInstallPhase::StoreBootstrapInFlight.admits_retained_root_repair());
    assert!(!FleetSubnetRootInstallPhase::InfrastructureVerified.admits_retained_root_repair());
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
        None,
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
            receipt.wasm_store = Principal::from_slice(&[46]);
        },
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.pool_canister = Principal::from_slice(&[43]);
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
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.component_topology_sha256[0] ^= 1,
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.root_plan_sha256[0] ^= 1,
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.placement_subnet = SubnetId::from_principal(Principal::from_slice(&[42; 29]));
        },
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.authority_journal_sequence += 1,
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.retained_journal_module_sha256 = [26; 32];
        },
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.retained_journal_wasm_size_bytes += 1;
        },
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.upgrade_predecessor_module_sha256 = [22; 32];
        },
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.upgrade_predecessor_wasm_size_bytes += 1;
        },
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.retained_journal_candid_sha256 = [23; 32];
        },
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.upgrade_predecessor_candid_sha256 = [21; 32];
        },
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.successor_module_sha256 = [28; 32];
        },
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.successor_candid_sha256 = [29; 32],
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.successor_wasm_size_bytes = MAX_REPAIR_WASM_BYTES as u64 + 1;
        },
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.required_pool_cycles += 1,
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.pool_policy_sha256[0] ^= 1,
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.top_up_fee_cycles += 1,
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.top_up_margin_cycles += 1,
    ] {
        let mut changed = receipt.clone();
        mutate(&mut changed);
        assert_invalid(&changed, &session, &current.journal);
    }

    receipt.successor_module_sha256 = receipt.upgrade_predecessor_module_sha256;
    assert_invalid(&receipt, &session, &current.journal);
}

#[test]
fn repair_receipt_publication_is_immutable_and_exactly_replayable() {
    let (current, session, receipt) = repair_authority_fixture();
    let resolved = ResolvedRetainedRootRepair {
        authority: receipt.clone(),
        terminal_receipt: None,
        needs_authority_publication: true,
        path: repair_authority_path(&current.path),
        successor_wasm_path: current.path.with_file_name("successor.wasm"),
    };
    publish_repair_candidate(&current.path, &resolved.authority)
        .expect("retain exact repair candidate before authority publication");
    publish_retained_root_repair(&resolved, &session, &current.journal)
        .expect("publish exact repair receipt");
    let replay = resolve_retained_root_repair(&current, &session, None, None)
        .expect("reload exact repair receipt")
        .expect("retained repair receipt");
    assert_eq!(replay.authority, receipt);
    assert!(!replay.needs_authority_publication);
}

#[test]
fn published_receipt_converges_asset_ready_operation_without_repeating_effects() {
    let (current, session, receipt) = repair_authority_fixture();
    let resolved = ResolvedRetainedRootRepair {
        authority: receipt,
        terminal_receipt: None,
        needs_authority_publication: true,
        path: repair_authority_path(&current.path),
        successor_wasm_path: current.path.with_file_name("successor.wasm"),
    };
    publish_repair_candidate(&current.path, &resolved.authority)
        .expect("retain exact repair candidate before authority publication");
    super::procedure::write_asset_ready_test_operation(&resolved)
        .expect("retain exact AssetReady interruption point");
    publish_retained_root_repair(&resolved, &session, &current.journal)
        .expect("publish immutable receipt before local operation converges");

    let replay = resolve_retained_root_repair(&current, &session, None, None)
        .expect("reload published repair")
        .expect("published repair receipt");
    assert!(!replay.needs_authority_publication);
    reconcile_published_retained_root_repair(&replay)
        .expect("receipt replay converges only the local operation");
    assert!(
        super::procedure::test_operation_is_adopted(&replay)
            .expect("read terminal repair operation")
    );
    reconcile_published_retained_root_repair(&replay)
        .expect("terminal operation replay is exact and idempotent");

    let mut conflicting_receipt = replay.authority.clone();
    conflicting_receipt.required_pool_cycles += 1;
    let conflicting = ResolvedRetainedRootRepair {
        authority: conflicting_receipt,
        terminal_receipt: replay.terminal_receipt,
        needs_authority_publication: false,
        path: replay.path,
        successor_wasm_path: replay.successor_wasm_path,
    };
    assert!(matches!(
        reconcile_published_retained_root_repair(&conflicting),
        Err(super::procedure::RetainedRootRepairProcedureError::InvalidDocument { .. })
    ));
}

#[test]
#[ignore = "the workspace runner supplies the governed serial PocketIC server"]
fn retained_repair_adoption_reaches_component_catalog_completion_and_closes_recovery() {
    let fixture = retained_repair_journey_fixture();
    let receipt = execute_exact_repair_and_reject_conflicts(&fixture);
    assert_unrelated_live_module_rejects(&fixture, &receipt);
    install_qualification_component(&fixture);
    complete_catalog_and_close_recovery(&fixture);
}

struct RetainedRepairJourneyFixture {
    root: PathBuf,
    pic: PocketIc,
    controller: Principal,
    coordinator: Principal,
    fleet_subnet_root: Principal,
    pool_canister: Principal,
    current: ResolvedFleetSubnetRootInstall,
    fleet_install_plan: crate::fleet_install_plan::PersistedFleetInstallPlan,
    session: FleetInstallSession,
    icp_context: InstallIcpContext,
    live_predecessor_path: PathBuf,
    successor_path: PathBuf,
    changed_path: PathBuf,
    wrong_candid_path: PathBuf,
    successor_module_sha256: [u8; 32],
    recovery_bundle_path: PathBuf,
}

struct RetainedRepairArtifacts {
    predecessor_wasm: Vec<u8>,
    live_predecessor_wasm: Vec<u8>,
    successor_wasm: Vec<u8>,
    predecessor_path: PathBuf,
    live_predecessor_path: PathBuf,
    successor_path: PathBuf,
    changed_path: PathBuf,
    wrong_candid_path: PathBuf,
}

#[expect(
    clippy::too_many_lines,
    reason = "the governed fixture binds one complete retained plan, live topology and isolated identity"
)]
fn retained_repair_journey_fixture() -> RetainedRepairJourneyFixture {
    let root = crate::test_support::temp_dir("retained-root-repair-pocketic-completion");
    fs::create_dir_all(&root).expect("create retained-repair qualification root");
    let artifacts = write_live_retained_repair_artifacts(&root);
    assert!(!wasm_exports_candid_pointer(&artifacts.predecessor_wasm));
    assert!(!wasm_exports_candid_pointer(
        &artifacts.live_predecessor_wasm
    ));
    let session = plan_qualification_session(&root);
    let mut pic = repair_pocket_ic();
    let (icp_executable, controller) = isolated_repair_identity(&root);
    let coordinator = pic.create_canister_with_settings(Some(controller), None);
    let fleet_subnet_root = pic.create_canister_with_settings(Some(controller), None);
    let root_subnet = pic
        .get_subnet(fleet_subnet_root)
        .expect("repair Root placement Subnet");
    let pool_canister = create_undersized_pool_asset(&pic, root_subnet, fleet_subnet_root);
    let wasm_store = pic.create_canister_on_subnet(None, None, root_subnet);
    let successor_module_sha256: [u8; 32] = Sha256::digest(&artifacts.successor_wasm).into();
    let (planned, fleet_install_plan) = crate::install_root::fleet_subnet_root_install_journal::tests::planned_repair_fixture_with_root_artifact(
        &root,
        &session,
        coordinator,
        |artifact| configure_repair_stub_artifact(&root, artifact, &artifacts),
    );
    let (planned, fleet_install_plan, wasm_store_wasm) =
        bind_finalized_repair_artifacts(&root, planned, fleet_install_plan);
    pic.set_controllers(wasm_store, None, vec![controller])
        .expect("grant fixture controller for exact Wasm Store installation");
    pic.install_canister(wasm_store, wasm_store_wasm, Vec::new(), Some(controller));
    pic.set_controllers(wasm_store, Some(controller), vec![fleet_subnet_root])
        .expect("retain exact Root-owned Wasm Store controllers");
    let current =
        store_bootstrapped_repair_checkpoint(planned, fleet_subnet_root, wasm_store, controller);
    assert_eq!(
        current.journal.phase,
        FleetSubnetRootInstallPhase::StoreBootstrapped
    );
    assert_eq!(current.journal.sequence, 15);
    let live = repair_live_state(&current, fleet_subnet_root);
    let root_authority =
        expected_root_authority(&current.journal).expect("compile exact retained Root authority");
    let pool_cycles = pic.cycle_balance(pool_canister);
    let init = RepairStubInit {
        authority: root_authority,
        pool_canister,
        pool_cycles,
        component_registry: live.observed_component_registry,
        store_bootstrap_operation_id: crate::install_root::root_store_bootstrap_operation_id(
            session.operation_id,
        ),
        registry_sync_operation_id: crate::install_root::root_registry_synchronization_operation_id(
            session.operation_id,
        ),
        store_bootstrap: current
            .journal
            .store_bootstrap
            .clone()
            .expect("retained Store bootstrap response"),
        registry_synchronization: live.registry_synchronization,
        joining_registry: live.joining_registry,
        active_registry: live.active_registry,
        joining_manifest: live.joining_manifest,
        active_manifest: live.active_manifest,
        joining_version: live.joining_version,
        active_version: live.active_version,
        join_response: live.join_response,
        root_acknowledgements: live.root_acknowledgements,
    };
    let init_bytes = candid::encode_one(init).expect("encode retained repair fixture authority");
    pic.install_canister(
        fleet_subnet_root,
        artifacts.live_predecessor_wasm.clone(),
        init_bytes.clone(),
        Some(controller),
    );
    pic.install_canister(
        coordinator,
        artifacts.live_predecessor_wasm.clone(),
        init_bytes,
        Some(controller),
    );
    let live_url = pic.make_live(None);
    let root_key = encode_hex(&pic.root_key().expect("PocketIC root key"));
    fund_repair_identity(
        &root,
        &controller,
        live_url.as_str(),
        &root_key,
        2_000_000_000_000,
    );
    let icp_context = InstallIcpContext::new(&icp_executable, &root, "local").with_local_replica(
        Some(LocalReplicaTarget {
            url: live_url.to_string(),
            root_key,
        }),
    );
    assert_eq!(
        pic.canister_status(fleet_subnet_root, Some(controller))
            .expect("read retained Root status")
            .module_hash,
        Some(Sha256::digest(&artifacts.live_predecessor_wasm).to_vec())
    );

    RetainedRepairJourneyFixture {
        root,
        pic,
        controller,
        coordinator,
        fleet_subnet_root,
        pool_canister,
        current,
        fleet_install_plan,
        session,
        icp_context,
        live_predecessor_path: artifacts.live_predecessor_path,
        successor_path: artifacts.successor_path,
        changed_path: artifacts.changed_path,
        wrong_candid_path: artifacts.wrong_candid_path,
        successor_module_sha256,
        recovery_bundle_path: crate::test_support::temp_dir("retained-root-repair-pocketic-bundle"),
    }
}

fn create_undersized_pool_asset(
    pic: &PocketIc,
    root_subnet: pocket_ic::common::rest::SubnetId,
    fleet_subnet_root: Principal,
) -> Principal {
    let pool_canister = pic
        .create_canister_with_params(
            None,
            CreateCanisterParams {
                cycles: Some(4_500_000_000_000),
                settings: None,
                placement: Some(CreateCanisterPlacement::SubnetId(root_subnet)),
            },
        )
        .expect("create exact undersized retained pool asset");
    pic.set_controllers(pool_canister, None, vec![fleet_subnet_root])
        .expect("retain exact Root-controlled imported pool asset");
    pool_canister
}

fn write_retained_repair_artifacts(root: &Path) -> RetainedRepairArtifacts {
    let compatible_candid = b"service : { ping: () -> (); }\n";
    let predecessor_wasm = hide_candid_pointer_export(minimal_canister_wasm(
        std::str::from_utf8(compatible_candid).expect("test Candid"),
        "predecessor",
    ));
    let live_predecessor_wasm = hide_candid_pointer_export(minimal_canister_wasm(
        std::str::from_utf8(compatible_candid).expect("test Candid"),
        "live-predecessor",
    ));
    let successor_wasm = hide_candid_pointer_export(minimal_canister_wasm(
        std::str::from_utf8(compatible_candid).expect("test Candid"),
        "successor",
    ));
    let changed_wasm = hide_candid_pointer_export(minimal_canister_wasm(
        std::str::from_utf8(compatible_candid).expect("test Candid"),
        "changed-successor",
    ));
    write_retained_repair_artifact_set(
        root,
        predecessor_wasm,
        live_predecessor_wasm,
        successor_wasm,
        changed_wasm,
        compatible_candid,
    )
}

fn write_live_retained_repair_artifacts(root: &Path) -> RetainedRepairArtifacts {
    let base = build_repair_stub_wasm();
    let extractor_source = root.join("repair-stub-build-output.wasm");
    fs::write(&extractor_source, &base).expect("write build-time Candid extractor source");
    let compatible_candid =
        extract_candid_bytes(&extractor_source).expect("extract build-time repair-stub Candid");
    fs::remove_file(&extractor_source).expect("remove disposable extractor source");
    let base = hide_candid_pointer_export(base);
    let predecessor_wasm = marked_wasm(&base, "predecessor");
    let live_predecessor_wasm = marked_wasm(&base, "live-predecessor");
    let successor_wasm = marked_wasm(&base, "successor");
    let changed_wasm = marked_wasm(&base, "changed-successor");
    write_retained_repair_artifact_set(
        root,
        predecessor_wasm,
        live_predecessor_wasm,
        successor_wasm,
        changed_wasm,
        &compatible_candid,
    )
}

fn write_retained_repair_artifact_set(
    root: &Path,
    predecessor_wasm: Vec<u8>,
    live_predecessor_wasm: Vec<u8>,
    successor_wasm: Vec<u8>,
    changed_wasm: Vec<u8>,
    compatible_candid: &[u8],
) -> RetainedRepairArtifacts {
    let wrong_candid = b"service : { ping: (nat) -> (); }\n";
    let wrong_candid_wasm = hide_candid_pointer_export(minimal_canister_wasm(
        std::str::from_utf8(wrong_candid).expect("wrong test Candid"),
        "wrong-candid",
    ));
    let predecessor_path = root.join("predecessor-root.wasm");
    let live_predecessor_path = root.join("live-predecessor-root.wasm");
    let successor_path = root.join("successor-root.wasm");
    let changed_path = root.join("changed-root.wasm");
    let wrong_candid_path = root.join("wrong-candid-root.wasm");
    fs::write(&predecessor_path, &predecessor_wasm).expect("write predecessor Wasm");
    fs::write(&live_predecessor_path, &live_predecessor_wasm).expect("write live predecessor Wasm");
    fs::write(&successor_path, &successor_wasm).expect("write successor Wasm");
    fs::write(&changed_path, &changed_wasm).expect("write changed successor Wasm");
    fs::write(&wrong_candid_path, &wrong_candid_wasm).expect("write wrong-Candid Wasm");
    for path in [
        &predecessor_path,
        &live_predecessor_path,
        &successor_path,
        &changed_path,
    ] {
        fs::write(path.with_extension("did"), compatible_candid)
            .expect("write exact compatible Candid sidecar");
    }
    fs::write(wrong_candid_path.with_extension("did"), wrong_candid)
        .expect("write exact wrong Candid sidecar");

    RetainedRepairArtifacts {
        predecessor_wasm,
        live_predecessor_wasm,
        successor_wasm,
        predecessor_path,
        live_predecessor_path,
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

fn repair_pocket_ic() -> PocketIc {
    let mut pic_builder = PocketIcBuilder::new()
        .with_nns_subnet()
        .with_ii_subnet()
        .with_application_subnet()
        .with_icp_features(IcpFeatures {
            cycles_token: Some(IcpFeaturesConfig::DefaultConfig),
            ..IcpFeatures::default()
        });
    if let Ok(server_url) = std::env::var("CANIC_POCKET_IC_SERVER_URL") {
        pic_builder =
            pic_builder.with_server_url(server_url.parse().expect("governed PocketIC server URL"));
    }
    let pic = pic_builder.build();
    pic.set_time(SystemTime::now().into());
    pic
}

#[derive(CandidType)]
struct RepairStubInit {
    authority: FleetSubnetRootAuthority,
    pool_canister: Principal,
    pool_cycles: u128,
    component_registry: RootComponentRegistryStatusResponse,
    store_bootstrap_operation_id: [u8; 32],
    registry_sync_operation_id: [u8; 32],
    store_bootstrap: RootStoreBootstrapResponse,
    registry_synchronization: RootRegistrySynchronizationOperationStatus,
    joining_registry: FleetRegistry,
    active_registry: FleetRegistry,
    joining_manifest: FleetRegistryManifest,
    active_manifest: FleetRegistryManifest,
    joining_version: FleetRegistryVersion,
    active_version: FleetRegistryVersion,
    join_response: FleetSubnetRootJoinResponse,
    root_acknowledgements: Vec<FleetSubnetRootSnapshotAcknowledgement>,
}

struct RetainedRepairLiveState {
    retained_component_registry: RootComponentRegistryStatusResponse,
    observed_component_registry: RootComponentRegistryStatusResponse,
    registry_synchronization: RootRegistrySynchronizationOperationStatus,
    joining_registry: FleetRegistry,
    active_registry: FleetRegistry,
    joining_manifest: FleetRegistryManifest,
    active_manifest: FleetRegistryManifest,
    joining_version: FleetRegistryVersion,
    active_version: FleetRegistryVersion,
    join_response: FleetSubnetRootJoinResponse,
    root_acknowledgements: Vec<FleetSubnetRootSnapshotAcknowledgement>,
}

fn configure_repair_stub_artifact(
    root: &Path,
    artifact: &mut CanicInfrastructureArtifactEntry,
    artifacts: &RetainedRepairArtifacts,
) {
    let candid = fs::read(artifacts.predecessor_path.with_extension("did"))
        .expect("read retained repair stub Candid sidecar");
    let role = CanisterRole::from("root");
    let capabilities = std::collections::BTreeSet::from([RoleCapabilityKey::Root]);
    let release_identity = artifact.protocol_release_identity.clone();
    let profile = derive_protocol_profile_hashes(&release_identity, &role, &capabilities, &candid);
    let artifact_relative_path = format!(
        ".canic/release-builds/{}/artifacts/fleet-subnet-root/root.wasm",
        artifact.release_build_id
    );
    let artifact_path = root.join(&artifact_relative_path);
    fs::create_dir_all(artifact_path.parent().expect("repair artifact parent"))
        .expect("create repair artifact parent");
    fs::write(&artifact_path, &artifacts.predecessor_wasm)
        .expect("write retained original Root artifact");
    fs::write(artifact_path.with_extension("did"), &candid)
        .expect("write exact retained Root Candid sidecar");
    let compressed = gzip_wasm(&artifacts.predecessor_wasm);
    let compressed_relative_path = format!("{artifact_relative_path}.gz");
    fs::write(root.join(&compressed_relative_path), &compressed)
        .expect("write retained Root gzip Wasm");
    artifact.protocol_role = role;
    artifact.protocol_capabilities = capabilities;
    artifact.wasm_relative_path = artifact_relative_path;
    artifact.wasm_size_bytes =
        u64::try_from(artifacts.predecessor_wasm.len()).expect("Root Wasm size");
    artifact.wasm_sha256_hex = encode_hex(&Sha256::digest(&artifacts.predecessor_wasm));
    artifact.wasm_gz_relative_path = compressed_relative_path;
    artifact.wasm_gz_size_bytes = u64::try_from(compressed.len()).expect("Root gzip Wasm size");
    artifact.wasm_gz_sha256_hex = encode_hex(&Sha256::digest(&compressed));
    artifact.candid_sha256 = profile.candid_sha256;
    artifact.protocol_profile_digest = profile.protocol_profile_digest;
}

fn bind_finalized_repair_artifacts(
    root: &Path,
    mut planned: ResolvedFleetSubnetRootInstall,
    mut plan: crate::fleet_install_plan::PersistedFleetInstallPlan,
) -> (
    ResolvedFleetSubnetRootInstall,
    crate::fleet_install_plan::PersistedFleetInstallPlan,
    Vec<u8>,
) {
    let release_build_id = plan.plan.release_build_id;
    let coordinator_wasm = minimal_canister_wasm("service : {}\n", "coordinator-artifact");
    let wasm_store_wasm = minimal_canister_wasm("service : {}\n", "wasm-store-artifact");
    let coordinator = write_test_infrastructure_artifact(
        root,
        release_build_id,
        CanicInfrastructureRole::FleetCoordinator,
        &coordinator_wasm,
        b"service : {}\n",
        0x41,
    );
    let wasm_store = write_test_infrastructure_artifact(
        root,
        release_build_id,
        CanicInfrastructureRole::WasmStore,
        &wasm_store_wasm,
        b"service : {}\n",
        0x43,
    );
    let root_artifact = planned.journal.root_artifact.clone();
    let mut entries = vec![coordinator, root_artifact.clone(), wasm_store.clone()];
    entries.sort_unstable_by_key(|entry| entry.role);
    let infrastructure = CanicInfrastructureArtifactManifest {
        release_build_id,
        entries,
    };
    let infrastructure_bytes = infrastructure
        .canonical_bytes()
        .expect("encode exact retained infrastructure manifest");
    let release_build_root = root
        .join(".canic/release-builds")
        .join(release_build_id.to_string());
    fs::write(
        release_build_root.join(INFRASTRUCTURE_ARTIFACT_MANIFEST_FILE),
        &infrastructure_bytes,
    )
    .expect("write exact retained infrastructure manifest");

    let component_wasm = minimal_canister_wasm("service : {}\n", "component-artifact");
    let component_candid = b"service : {}\n";
    let component_directory =
        format!(".canic/release-builds/{release_build_id}/artifacts/component");
    let component_wasm_relative_path = format!("{component_directory}/component.wasm");
    let component_gzip_relative_path = format!("{component_wasm_relative_path}.gz");
    let component_gzip = gzip_wasm(&component_wasm);
    fs::create_dir_all(root.join(&component_directory))
        .expect("create Component artifact directory");
    fs::write(root.join(&component_wasm_relative_path), &component_wasm)
        .expect("write Component raw Wasm");
    fs::write(root.join(&component_gzip_relative_path), &component_gzip)
        .expect("write Component gzip Wasm");
    fs::write(
        root.join(&component_wasm_relative_path)
            .with_extension("did"),
        component_candid,
    )
    .expect("write Component Candid sidecar");
    let application = ApplicationArtifactUnion {
        release_build_id,
        fleet_component_topology_digest: canic_core::ids::ComponentTopologyDigest::from_bytes(
            [0x44; 32],
        ),
        entries: vec![ApplicationArtifactEntry {
            role: CanisterRole::from("project_hub"),
            package: "project-hub".to_string(),
            release_build_id,
            wasm_relative_path: component_wasm_relative_path,
            wasm_size_bytes: component_wasm.len() as u64,
            wasm_sha256_hex: encode_hex(&Sha256::digest(&component_wasm)),
            wasm_gz_relative_path: component_gzip_relative_path,
            wasm_gz_size_bytes: component_gzip.len() as u64,
            wasm_gz_sha256_hex: encode_hex(&Sha256::digest(&component_gzip)),
            candid_sha256: Sha256::digest(component_candid).into(),
            protocol_profile_digest: ProtocolProfileDigest::from_bytes([0x45; 32]),
        }],
    };
    application
        .validate_retained_shape()
        .expect("validate exact retained application artifact union");
    let application_bytes =
        serde_json::to_vec(&application).expect("encode retained application artifact union");
    fs::write(
        release_build_root.join(APPLICATION_ARTIFACT_UNION_FILE),
        &application_bytes,
    )
    .expect("write exact retained application artifact union");

    plan.plan.application_artifact_union_digest = Sha256::digest(&application_bytes).into();
    let plan_bytes = serde_json::to_vec(&plan.plan).expect("encode artifact-bound Fleet plan");
    plan.digest = Sha256::digest(&plan_bytes).into();
    fs::write(&plan.path, &plan_bytes).expect("retain artifact-bound Fleet plan");
    planned.journal.fleet_install_plan_digest = plan.digest;
    planned.journal.infrastructure_manifest_digest = Sha256::digest(&infrastructure_bytes).into();
    planned.journal.root_artifact = root_artifact;
    planned.journal.wasm_store_artifact = wasm_store;
    planned.journal.expected_wasm_store_module_hash = Sha256::digest(&wasm_store_wasm).into();
    fs::write(
        &planned.path,
        serde_json::to_vec(&planned.journal).expect("encode artifact-bound Root journal"),
    )
    .expect("retain artifact-bound Root journal");
    (planned, plan, wasm_store_wasm)
}

fn write_test_infrastructure_artifact(
    root: &Path,
    release_build_id: canic_core::ids::ReleaseBuildId,
    role: CanicInfrastructureRole,
    wasm: &[u8],
    candid: &[u8],
    marker: u8,
) -> CanicInfrastructureArtifactEntry {
    let directory = format!(
        ".canic/release-builds/{release_build_id}/artifacts/{}",
        role.as_str()
    );
    let wasm_relative_path = format!("{directory}/{}.wasm", role.as_str());
    let wasm_gz_relative_path = format!("{wasm_relative_path}.gz");
    let wasm_gz = gzip_wasm(wasm);
    fs::create_dir_all(root.join(&directory)).expect("create infrastructure artifact directory");
    fs::write(root.join(&wasm_relative_path), wasm).expect("write infrastructure raw Wasm");
    fs::write(root.join(&wasm_gz_relative_path), &wasm_gz).expect("write infrastructure gzip Wasm");
    fs::write(root.join(&wasm_relative_path).with_extension("did"), candid)
        .expect("write infrastructure Candid sidecar");
    CanicInfrastructureArtifactEntry {
        role,
        package: format!("canic-{}", role.as_str().replace('_', "-")),
        protocol_release_identity: "0.109.10-test".to_string(),
        protocol_role: CanisterRole::owned(role.protocol_role_name().to_string()),
        protocol_capabilities: std::collections::BTreeSet::new(),
        release_build_id,
        wasm_relative_path,
        wasm_size_bytes: wasm.len() as u64,
        wasm_sha256_hex: encode_hex(&Sha256::digest(wasm)),
        wasm_gz_relative_path,
        wasm_gz_size_bytes: wasm_gz.len() as u64,
        wasm_gz_sha256_hex: encode_hex(&Sha256::digest(&wasm_gz)),
        candid_sha256: Sha256::digest(candid).into(),
        protocol_profile_digest: ProtocolProfileDigest::from_bytes([marker; 32]),
    }
}

fn gzip_wasm(wasm: &[u8]) -> Vec<u8> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(wasm).expect("gzip test Wasm");
    encoder.finish().expect("finish test Wasm gzip")
}

fn store_bootstrapped_repair_checkpoint(
    planned: ResolvedFleetSubnetRootInstall,
    fleet_subnet_root: Principal,
    wasm_store: Principal,
    installation_controller: Principal,
) -> ResolvedFleetSubnetRootInstall {
    let creating = begin_root_creation(&planned, installation_controller)
        .expect("retain Root creation intent");
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
    .expect("retain installed predecessor Root");
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
    let mut temporary_controllers = vec![installation_controller, fleet_subnet_root];
    temporary_controllers.sort();
    let adopted = record_store_adopted(
        &adopting,
        FleetSubnetWasmStoreAdoptionResponse {
            operation_id: crate::install_root::root_store_adoption_operation_id(
                adopting.journal.install_operation_id,
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
    let journal_directory = bootstrapped
        .path
        .parent()
        .expect("retained Root journal directory");
    for (file, bytes) in [
        ("root-create-result.json", b"{}".as_slice()),
        ("wasm-store-create-result.json", b"{}".as_slice()),
        ("wasm-store-install-args.bin", b"store".as_slice()),
        ("root-install-args.bin", b"root".as_slice()),
    ] {
        fs::write(journal_directory.join(file), bytes)
            .expect("retain phase-derived Root recovery sidecar");
    }
    assert!(bootstrapped.advanced);
    bootstrapped
}

#[expect(
    clippy::too_many_lines,
    reason = "the governed fixture compiles every exact retained and observed Registry authority"
)]
fn repair_live_state(
    current: &ResolvedFleetSubnetRootInstall,
    fleet_subnet_root: Principal,
) -> RetainedRepairLiveState {
    let genesis = FleetRegistryOps::compile_genesis(
        &current.journal.authority.binding.fleet.app,
        current.journal.authority.clone(),
        &current.journal.component_topology,
        crate::test_support::fleet_admission_policy(
            current.journal.authority.binding.fleet.clone(),
        ),
    )
    .expect("compile retained recovery genesis Registry");
    let joining_entry =
        expected_registry_join_entry(&current.journal).expect("retained Root Registry entry");
    let joining = FleetRegistryOps::compile_joining(
        &current.journal.authority,
        &current.journal.component_topology,
        &genesis,
        joining_entry.clone(),
    )
    .expect("compile retained recovery joining Registry");
    let active = FleetRegistryOps::compile_active(
        &current.journal.authority,
        &current.journal.component_topology,
        &joining,
    )
    .expect("compile retained recovery active Registry");
    let active_version = FleetRegistryOps::version(
        &current.journal.authority,
        &current.journal.component_topology,
        &active,
    )
    .expect("compile retained recovery active Registry version");
    let joining_version = FleetRegistryOps::version(
        &current.journal.authority,
        &current.journal.component_topology,
        &joining,
    )
    .expect("compile retained recovery joining Registry version");
    let joining_manifest = FleetRegistryOps::manifest(
        &current.journal.authority,
        &current.journal.component_topology,
        &joining,
    )
    .expect("compile retained recovery joining Registry manifest");
    let active_manifest = FleetRegistryOps::manifest(
        &current.journal.authority,
        &current.journal.component_topology,
        &active,
    )
    .expect("compile retained recovery active Registry manifest");
    let acknowledgement = FleetSubnetRootSnapshotAcknowledgement {
        fleet_subnet_root,
        version: joining_version.clone(),
    };
    let synchronization = FleetSubnetRootRegistrySyncResponse {
        fleet_subnet_root,
        version: joining_version.clone(),
        acknowledgement: acknowledgement.clone(),
    };
    let directory = FleetRegistryOps::directory_for_root(
        &current.journal.authority,
        &current.journal.component_topology,
        &active,
        fleet_subnet_root,
    )
    .expect("compile retained recovery Root Directory");
    let activation = FleetSubnetRootRegistryMirrorActivationResponse {
        fleet_subnet_root,
        previous_registry: joining_version.clone(),
        version: active_version.clone(),
        directory,
    };
    let retained = RootComponentRegistryStatusResponse {
        fleet_subnet_root,
        prepared_against_registry: active_version.clone(),
        release_set: FleetSubnetRootReleaseSet {
            release_build_id: current.journal.release_build_id,
            manifest_digest: current
                .journal
                .root_plan
                .initial_release_set
                .manifest_digest,
        },
        component_topology_digest: current.journal.root_plan.component_topology_digest,
        next_allocation_sequence: 1,
        reserved_component_instances: 0,
        committed_component_instances: 0,
        managed_descendants: 0,
        known_created_component_canisters: 0,
        encoded_bytes: 0,
        initial_inventory: None,
    };
    let observed = RootComponentRegistryStatusResponse {
        next_allocation_sequence: 2,
        reserved_component_instances: 1,
        encoded_bytes: 1_024,
        ..retained.clone()
    };
    RetainedRepairLiveState {
        retained_component_registry: retained,
        observed_component_registry: observed,
        registry_synchronization: RootRegistrySynchronizationOperationStatus {
            synchronization,
            activation: Some(activation),
        },
        joining_registry: joining,
        active_registry: active,
        joining_manifest,
        active_manifest,
        joining_version: joining_version.clone(),
        active_version,
        join_response: FleetSubnetRootJoinResponse {
            entry: joining_entry,
            version: joining_version,
        },
        root_acknowledgements: vec![acknowledgement],
    }
}

fn build_repair_stub_wasm() -> Vec<u8> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("Canic workspace root");
    let target_dir = workspace_root.join("target/pic-wasm");
    let output = Command::new("cargo")
        .current_dir(workspace_root)
        .env("CARGO_TARGET_DIR", &target_dir)
        .args([
            "build",
            "--locked",
            "--offline",
            "--target",
            "wasm32-unknown-unknown",
            "--profile",
            "fast",
            "-p",
            "retained_root_repair_stub",
        ])
        .output()
        .expect("launch retained repair stub build");
    assert!(
        output.status.success(),
        "retained repair stub build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read(
        target_dir
            .join("wasm32-unknown-unknown/fast")
            .join("retained_root_repair_stub.wasm"),
    )
    .expect("read retained repair stub Wasm")
}

fn marked_wasm(base: &[u8], marker: &str) -> Vec<u8> {
    let mut wasm = base.to_vec();
    append_custom_section(&mut wasm, "canic:retained-repair-test", marker.as_bytes());
    wasm
}

fn isolated_repair_identity(root: &Path) -> (String, Principal) {
    let config = root.join("icp-config");
    let data = root.join("icp-data");
    fs::create_dir_all(&config).expect("create isolated ICP identity config");
    fs::create_dir_all(&data).expect("create isolated ICP identity data");
    let created = Command::new("icp")
        .env_remove("ICP_ENVIRONMENT")
        .env("XDG_CONFIG_HOME", &config)
        .env("XDG_DATA_HOME", &data)
        .args([
            "identity",
            "new",
            "--storage",
            "plaintext",
            "--quiet",
            "repair-controller",
        ])
        .output()
        .expect("create isolated repair identity");
    assert!(
        created.status.success(),
        "isolated ICP identity creation failed: {}",
        String::from_utf8_lossy(&created.stderr)
    );
    let principal = Command::new("icp")
        .env_remove("ICP_ENVIRONMENT")
        .env("XDG_CONFIG_HOME", &config)
        .env("XDG_DATA_HOME", &data)
        .args(["identity", "principal", "--identity", "repair-controller"])
        .output()
        .expect("read isolated repair identity Principal");
    assert!(
        principal.status.success(),
        "isolated ICP Principal lookup failed: {}",
        String::from_utf8_lossy(&principal.stderr)
    );
    let principal = Principal::from_text(String::from_utf8_lossy(&principal.stdout).trim())
        .expect("isolated repair Principal");
    let wrapper = root.join("icp-repair-controller");
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nXDG_CONFIG_HOME='{}'\nXDG_DATA_HOME='{}'\nunset ICP_ENVIRONMENT\nexport XDG_CONFIG_HOME XDG_DATA_HOME\nexec icp \"$@\" --identity repair-controller\n",
            config.display(),
            data.display()
        ),
    )
    .expect("write isolated ICP wrapper");
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700))
        .expect("make isolated ICP wrapper executable");
    (wrapper.to_string_lossy().into_owned(), principal)
}

fn fund_repair_identity(
    root: &Path,
    controller: &Principal,
    network_url: &str,
    root_key: &str,
    cycles: u128,
) {
    let output = Command::new("icp")
        .env_remove("ICP_ENVIRONMENT")
        .env("XDG_CONFIG_HOME", root.join("icp-config"))
        .env("XDG_DATA_HOME", root.join("icp-data"))
        .args([
            "cycles",
            "transfer",
            &cycles.to_string(),
            &controller.to_text(),
            "--identity",
            "anonymous",
            "-n",
            network_url,
            "-k",
            root_key,
        ])
        .output()
        .expect("fund isolated repair identity");
    assert!(
        output.status.success(),
        "isolated repair identity funding failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[expect(
    clippy::too_many_lines,
    reason = "the governed journey deliberately enumerates and restarts every canonical Root phase"
)]
fn advance_repair_checkpoint_through_canonical_journal(
    fixture: &RetainedRepairJourneyFixture,
) -> ResolvedFleetSubnetRootInstall {
    let store_request = RootStoreBootstrapRequest {
        operation_id: crate::install_root::root_store_bootstrap_operation_id(
            fixture.session.operation_id,
        ),
        manifest_payload_size_bytes: 1,
    };
    let store_verified = restart_root_journal(
        crate::install_root::fleet_subnet_root_store_bootstrap::verify_retained_store_bootstrap(
            &fixture.icp_context,
            &fixture.current,
            store_request.clone(),
        )
        .expect("production Store owner re-observes exact retained bootstrap"),
    );
    let topology = store_verified.journal.component_topology.clone();
    let authority = store_verified.journal.authority.clone();
    let genesis = FleetRegistryOps::compile_genesis(
        &authority.binding.fleet.app,
        authority.clone(),
        &topology,
        crate::test_support::fleet_admission_policy(authority.binding.fleet.clone()),
    )
    .expect("recompile retained genesis Registry");
    let join_entry = expected_registry_join_entry(&store_verified.journal)
        .expect("recompile retained Root Registry entry");
    let joining_registry =
        FleetRegistryOps::compile_joining(&authority, &topology, &genesis, join_entry)
            .expect("recompile retained joining Registry");
    let joining_version = FleetRegistryOps::version(&authority, &topology, &joining_registry)
        .expect("recompile retained joining version");
    let binding = resolve_infrastructure_protocol_binding(
        &fixture.root,
        "local",
        &store_verified.journal.root_artifact,
    )
    .expect("resolve production repair-stub protocol binding");
    crate::install_root::fleet_subnet_root_registry_join::drive_registry_join(
        &fixture.icp_context,
        &binding,
        &topology,
        store_verified.clone(),
        &genesis,
        &joining_registry,
    )
    .expect("production Registry join owner re-observes exact successor");
    let join_verified = restart_root_journal_path(&store_verified.path);
    let sync_request = FleetSubnetRootRegistrySyncRequest {
        operation_id: crate::install_root::root_registry_synchronization_operation_id(
            fixture.session.operation_id,
        ),
        expected_registry: joining_version.clone(),
        store_bootstrap: store_request.clone(),
    };
    crate::install_root::fleet_subnet_root_registry_sync::drive_root_sync(
        &fixture.icp_context,
        join_verified.clone(),
        sync_request,
    )
    .expect("production Registry synchronization owner re-observes exact successor");
    let sync_verified = restart_root_journal_path(&join_verified.path);
    let activation = crate::install_root::fleet_registry_activation_journal::plan_fleet_registry_activation(
        crate::install_root::fleet_registry_activation_journal::PlanFleetRegistryActivationRequest {
            fleet_install_plan: &fixture.fleet_install_plan,
            component_topology: topology.clone(),
            joining_registry: joining_registry.clone(),
        },
    );
    let activation = activation.expect("plan production Registry activation journal");
    let activated = crate::install_root::fleet_registry_activation::drive_activation(
        fixture.icp_context.cli(),
        &binding,
        fixture.coordinator,
        &topology,
        vec![fixture.fleet_subnet_root],
        activation,
    )
    .expect("production Registry activation owner converges exact successor");
    let active_registry =
        FleetRegistryOps::compile_active(&authority, &topology, &joining_registry)
            .expect("recompile retained active Registry");
    assert_eq!(activated.journal.active_registry, active_registry);
    let active_version = FleetRegistryOps::version(&authority, &topology, &active_registry)
        .expect("recompile retained active version");
    let directory = FleetRegistryOps::directory_for_root(
        &authority,
        &topology,
        &active_registry,
        fixture.fleet_subnet_root,
    )
    .expect("recompile retained Root Directory");
    let mirror_request = FleetSubnetRootRegistryMirrorActivationRequest {
        previous_registry: joining_version,
        expected_registry: active_version.clone(),
        expected_directory: directory,
        store_bootstrap: store_request.clone(),
    };
    crate::install_root::fleet_subnet_root_registry_mirror_activation::drive_root_mirror_activation(
        &fixture.icp_context,
        sync_verified.clone(),
        mirror_request,
    )
    .expect("production Registry mirror owner re-observes exact successor");
    let mirror_verified = restart_root_journal_path(&sync_verified.path);
    let preparation_request = RootComponentRegistryPreparationRequest {
        store_bootstrap: store_request,
        expected_fleet_registry: active_version,
    };
    crate::install_root::fleet_subnet_root_component_registry_preparation::drive_component_registry_preparation(
        &fixture.icp_context,
        mirror_verified.clone(),
        preparation_request,
    )
    .expect("production Component Registry owner re-observes exact successor");
    restart_root_journal_path(&mirror_verified.path)
}

fn restart_root_journal(current: ResolvedFleetSubnetRootInstall) -> ResolvedFleetSubnetRootInstall {
    let bytes = fs::read(&current.path).expect("read durable Root journal after interruption");
    let journal = serde_json::from_slice::<FleetSubnetRootInstallJournal>(&bytes)
        .expect("decode durable Root journal after interruption");
    assert_eq!(journal, current.journal);
    ResolvedFleetSubnetRootInstall {
        journal,
        path: current.path,
        advanced: false,
    }
}

fn restart_root_journal_path(path: &Path) -> ResolvedFleetSubnetRootInstall {
    let bytes = fs::read(path).expect("read durable Root journal after production phase replay");
    let journal = serde_json::from_slice::<FleetSubnetRootInstallJournal>(&bytes)
        .expect("decode durable Root journal after production phase replay");
    ResolvedFleetSubnetRootInstall {
        journal,
        path: path.to_path_buf(),
        advanced: false,
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one governed sequence binds repair effects, exact replay and negative authority checks"
)]
fn execute_exact_repair_and_reject_conflicts(
    fixture: &RetainedRepairJourneyFixture,
) -> RetainedRootRepairReceiptV1 {
    let recovery_bundle = FleetInstallRecoveryBundleCheckpoint::persistent_at(
        &fixture.root,
        &fixture.session,
        &fixture.fleet_install_plan,
        fixture.recovery_bundle_path.clone(),
    );
    let (repair, root_binding) = resolve_repair_for_execution(fixture);
    recovery_bundle
        .checkpoint()
        .expect("checkpoint candidate and both Candid sidecars before authority publication");
    assert!(repair_candidate_path(&fixture.current.path).is_file());
    assert!(!repair_authority_path(&fixture.current.path).exists());
    crate::install_root::verify_fleet_install_recovery_bundle(&fixture.recovery_bundle_path)
        .expect("verify the effect-equivalent preflight bundle before authority publication");
    publish_retained_root_repair_authority(&repair, &fixture.session, &fixture.current.journal)
        .expect("publish provisional authority before repair effects");
    recovery_bundle
        .checkpoint()
        .expect("checkpoint published authority before repair effects");
    let operator_before = fixture
        .icp_context
        .cli()
        .identity_cycles_balance()
        .expect("observe pre-repair operator cycles");
    let asset_before = fixture.pic.cycle_balance(fixture.pool_canister);
    let operation = execute_retained_root_repair(
        &fixture.icp_context,
        &root_binding,
        &repair,
        &repair.successor_wasm_path,
        &recovery_bundle,
    )
    .unwrap_or_else(|error| {
        let operator_after = fixture
            .icp_context
            .cli()
            .identity_cycles_balance()
            .expect("observe operator balance after failed repair");
        let asset_after = fixture.pic.cycle_balance(fixture.pool_canister);
        panic!(
            "execute exact Root upgrade, funding and pool reinspection: {error}; operator_before={operator_before}; operator_after={operator_after}; asset_before={asset_before}; asset_after={asset_after}"
        );
    });
    let operator_after_effects = fixture
        .icp_context
        .cli()
        .identity_cycles_balance()
        .expect("observe repaired operator balance");
    let asset_after_effects = fixture.pic.cycle_balance(fixture.pool_canister);
    let replayed_operation = execute_retained_root_repair(
        &fixture.icp_context,
        &root_binding,
        &repair,
        &repair.successor_wasm_path,
        &recovery_bundle,
    )
    .expect("resume when the exact successor and repaired pool asset are already live");
    assert_eq!(replayed_operation, operation);
    assert_eq!(
        fixture
            .icp_context
            .cli()
            .identity_cycles_balance()
            .expect("observe effect-free successor replay"),
        operator_after_effects
    );
    let asset_after_replay = fixture.pic.cycle_balance(fixture.pool_canister);
    assert!(asset_after_replay <= asset_after_effects);
    assert!(asset_after_replay >= 5_000_000_000_000);
    assert!(matches!(
        publish_retained_root_repair_receipt(&repair, &fixture.session, &fixture.current.journal),
        Err(RetainedRootRepairError::PrematureTerminalReceipt)
    ));
    let advanced = advance_repair_checkpoint_through_canonical_journal(fixture);
    assert_eq!(
        advanced.journal.phase,
        FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified
    );
    assert_eq!(advanced.journal.sequence, 28);
    let repair =
        resolve_retained_root_repair(&advanced, &fixture.session, None, Some(5_000_000_000_000))
            .expect("reload provisional authority after canonical journal replay")
            .expect("retained repair remains active");
    verify_retained_component_registry_preparation(&fixture.icp_context, &advanced.journal)
        .expect("verify advanced Component Registry through status-like replay");
    assert_eq!(
        fixture
            .pic
            .canister_status(fixture.fleet_subnet_root, Some(fixture.controller))
            .expect("read final repaired Root status")
            .module_hash,
        Some(fixture.successor_module_sha256.to_vec())
    );
    publish_retained_root_repair(&repair, &fixture.session, &advanced.journal)
        .expect("publish exact repair receipt after live PocketIC verification");
    recovery_bundle
        .checkpoint()
        .expect("checkpoint the terminal repair receipt before reconciliation");
    reconcile_published_retained_root_repair(&repair)
        .expect("make the exact repair operation terminal");
    recovery_bundle
        .checkpoint()
        .expect("checkpoint the reconciled terminal repair operation");
    assert!(
        super::procedure::test_operation_is_adopted(&repair)
            .expect("read terminal production repair operation")
    );
    assert_repair_replay_has_no_effect(fixture, &repair, &operation, operator_before);

    let changed = repair_adoption_for_pool(
        fixture.fleet_subnet_root,
        fixture.pool_canister,
        &fixture.live_predecessor_path,
        &fixture.changed_path,
    );
    assert!(matches!(
        resolve_retained_root_repair(
            &fixture.current,
            &fixture.session,
            Some(&changed),
            Some(5_000_000_000_000),
        ),
        Err(RetainedRootRepairError::ConflictingAuthority { .. })
    ));
    let mut wrong_authority = fixture.session.clone();
    wrong_authority.operation_id[0] ^= 1;
    assert!(matches!(
        resolve_retained_root_repair(&fixture.current, &wrong_authority, None, None),
        Err(RetainedRootRepairError::InvalidDocument { .. })
    ));
    repair.authority
}

fn resolve_repair_for_execution(
    fixture: &RetainedRepairJourneyFixture,
) -> (
    ResolvedRetainedRootRepair,
    crate::protocol_binding::ResolvedProtocolBinding,
) {
    let wrong_candid = repair_adoption_for_pool(
        fixture.fleet_subnet_root,
        fixture.pool_canister,
        &fixture.live_predecessor_path,
        &fixture.wrong_candid_path,
    );
    assert!(matches!(
        resolve_retained_root_repair(
            &fixture.current,
            &fixture.session,
            Some(&wrong_candid),
            Some(5_000_000_000_000),
        ),
        Err(RetainedRootRepairError::CandidMismatch(_))
    ));

    let adoption = repair_adoption_for_pool(
        fixture.fleet_subnet_root,
        fixture.pool_canister,
        &fixture.live_predecessor_path,
        &fixture.successor_path,
    );
    let repair = resolve_retained_root_repair(
        &fixture.current,
        &fixture.session,
        Some(&adoption),
        Some(5_000_000_000_000),
    )
    .expect("compile exact repair adoption")
    .expect("repair candidate");
    assert_eq!(
        repair.authority.successor_module_hash(),
        fixture.successor_module_sha256
    );
    let root_binding = resolve_infrastructure_protocol_binding(
        &fixture.root,
        "local",
        &fixture.current.journal.root_artifact,
    )
    .expect("resolve exact retained Root protocol binding");
    verify_pre_repair_root_authority(
        &fixture.icp_context,
        &root_binding,
        &fixture.current.journal,
        &repair.authority,
    )
    .expect("verify live predecessor controller, module and Fleet authority");
    (repair, root_binding)
}

fn assert_repair_replay_has_no_effect(
    fixture: &RetainedRepairJourneyFixture,
    repair: &ResolvedRetainedRootRepair,
    operation: &super::procedure::RetainedRootRepairOperationV1,
    operator_before: u128,
) {
    let operator_after = fixture
        .icp_context
        .cli()
        .identity_cycles_balance()
        .expect("observe post-repair operator cycles");
    let asset_after = fixture.pic.cycle_balance(fixture.pool_canister);
    assert!(asset_after >= 5_000_000_000_000);
    let (funding_attempts, operator_debit_cycles, asset_credit_cycles) = operation
        .test_funding_reconciliation()
        .expect("read exact retained repair funding observations");
    assert_eq!(funding_attempts, 1);
    assert_eq!(operator_before - operator_after, operator_debit_cycles);
    assert_eq!(
        operator_debit_cycles,
        asset_credit_cycles + RETAINED_ROOT_REPAIR_TOP_UP_FEE_CYCLES
    );
    reconcile_published_retained_root_repair(repair)
        .expect("terminal receipt replay remains effect-free");
    let asset_after_replay = fixture.pic.cycle_balance(fixture.pool_canister);
    assert!(asset_after_replay <= asset_after);
    assert!(asset_after_replay >= 5_000_000_000_000);
    assert_eq!(
        fixture
            .icp_context
            .cli()
            .identity_cycles_balance()
            .expect("observe replay operator cycles"),
        operator_after
    );
}

fn assert_unrelated_live_module_rejects(
    fixture: &RetainedRepairJourneyFixture,
    receipt: &RetainedRootRepairReceiptV1,
) {
    let unrelated_wasm = minimal_canister_wasm("service : {}\n", "unrelated-root");
    fixture
        .pic
        .upgrade_canister(
            fixture.fleet_subnet_root,
            unrelated_wasm,
            Vec::new(),
            Some(fixture.controller),
        )
        .expect("temporarily install unrelated live Root module");
    assert!(
        require_expected_module_hash(
            fixture.icp_context.cli(),
            fixture.fleet_subnet_root,
            receipt.successor_module_hash(),
            "Fleet Subnet Root repaired successor",
        )
        .is_err(),
        "an unrelated live Root module must fail exact repair verification"
    );

    fixture
        .pic
        .upgrade_canister(
            fixture.fleet_subnet_root,
            fs::read(&fixture.successor_path).expect("read exact successor Wasm"),
            candid::encode_one(()).expect("encode successor upgrade args"),
            Some(fixture.controller),
        )
        .expect("restore the exact repaired successor Root");
    require_expected_module_hash(
        fixture.icp_context.cli(),
        fixture.fleet_subnet_root,
        receipt.successor_module_hash(),
        "Fleet Subnet Root repaired successor",
    )
    .expect("exact repaired successor verifies after unrelated-module rejection");
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

    let recovery_bundle = FleetInstallRecoveryBundleCheckpoint::persistent_at(
        &fixture.root,
        &fixture.session,
        &fixture.fleet_install_plan,
        fixture.recovery_bundle_path.clone(),
    );
    let bundle = recovery_bundle
        .checkpoint()
        .expect("checkpoint terminal retained-repair bundle");
    let detached_source = fixture.root.with_extension("deleted-source");
    fs::rename(&fixture.root, &detached_source)
        .expect("remove the original source workspace without deleting its evidence");
    crate::install_root::import_fleet_install_recovery_bundle(&bundle, &fixture.root)
        .expect("verify and import the exact bundle after source-workspace removal");
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

fn repair_adoption(
    root: Principal,
    live_predecessor_wasm: &Path,
    successor_wasm: &Path,
) -> RetainedRootRepairAdoption {
    repair_adoption_for_pool(
        root,
        Principal::from_slice(&[45]),
        live_predecessor_wasm,
        successor_wasm,
    )
}

fn repair_adoption_for_pool(
    root: Principal,
    pool_canister: Principal,
    live_predecessor_wasm: &Path,
    successor_wasm: &Path,
) -> RetainedRootRepairAdoption {
    RetainedRootRepairAdoption::from_str(&format!(
        "{root},{}={},{}",
        pool_canister,
        live_predecessor_wasm.display(),
        successor_wasm.display()
    ))
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
    current.journal.wasm_store = Some(Principal::from_slice(&[45]));
    current.journal.installation_controller = Some(controller);
    let retained_component_registry =
        repair_live_state(&current, root_canister).retained_component_registry;
    current.journal.component_registry_preparation_request =
        Some(RootComponentRegistryPreparationRequest {
            store_bootstrap: RootStoreBootstrapRequest {
                operation_id: [0x31; 32],
                manifest_payload_size_bytes: 1,
            },
            expected_fleet_registry: retained_component_registry
                .prepared_against_registry
                .clone(),
        });
    current.journal.component_registry_preparation_response = Some(retained_component_registry);
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
        validate_receipt(Path::new("receipt.json"), receipt, session, journal, None),
        Err(RetainedRootRepairError::InvalidDocument { .. })
    ));
}

fn fixture_receipt(
    session: &FleetInstallSession,
    journal: &FleetSubnetRootInstallJournal,
) -> RetainedRootRepairReceiptV1 {
    let fleet_subnet_root = journal.fleet_subnet_root.expect("Root");
    let pool_canister = Principal::from_slice(&[45]);
    let upgrade_predecessor_module_sha256 = [9; 32];
    let successor_module_sha256 = [8; 32];
    let transition = RetainedRootRepairTransition {
        pool_canister,
        upgrade_predecessor_module_sha256,
        upgrade_predecessor_wasm_size_bytes: 1,
        upgrade_predecessor_candid_sha256: journal.root_artifact.candid_sha256,
        successor_module_sha256,
        successor_wasm_size_bytes: 1,
        successor_candid_sha256: journal.root_artifact.candid_sha256,
        required_pool_cycles: 5_000_000_000_000,
    };
    RetainedRootRepairReceiptV1 {
        schema_version: 1,
        repair_operation_id: repair_operation_id(
            session,
            journal,
            &transition,
            journal.phase,
            journal.sequence,
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
        component_topology_sha256: domain_digest(
            b"canic.root-repair.component-topology.v1\0",
            &journal.component_topology,
        )
        .expect("component topology digest"),
        root_plan_sha256: domain_digest(b"canic.root-repair.root-plan.v1\0", &journal.root_plan)
            .expect("Root plan digest"),
        install_operation_id: session.operation_id,
        authority: journal.authority.clone(),
        placement_subnet: journal.root_plan.placement_subnet,
        fleet_subnet_root,
        wasm_store: journal.wasm_store.expect("Wasm Store"),
        pool_canister,
        installation_controller: journal.installation_controller.expect("controller"),
        authority_journal_phase: journal.phase,
        authority_journal_sequence: journal.sequence,
        retained_journal_module_sha256: journal.expected_root_module_hash,
        retained_journal_wasm_size_bytes: journal.root_artifact.wasm_size_bytes,
        upgrade_predecessor_module_sha256,
        upgrade_predecessor_wasm_size_bytes: 1,
        successor_module_sha256,
        successor_wasm_size_bytes: 1,
        retained_journal_candid_sha256: journal.root_artifact.candid_sha256,
        upgrade_predecessor_candid_sha256: journal.root_artifact.candid_sha256,
        successor_candid_sha256: journal.root_artifact.candid_sha256,
        required_pool_cycles: 5_000_000_000_000,
        pool_policy_sha256: repair_pool_policy_sha256(journal).expect("pool policy digest"),
        top_up_fee_cycles: RETAINED_ROOT_REPAIR_TOP_UP_FEE_CYCLES,
        top_up_margin_cycles: RETAINED_ROOT_REPAIR_TOP_UP_MARGIN_CYCLES,
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

fn hide_candid_pointer_export(mut wasm: Vec<u8>) -> Vec<u8> {
    const EXPORTED: &[u8; 18] = b"get_candid_pointer";
    const HIDDEN: &[u8; 18] = b"old_candid_pointer";
    let matches = wasm
        .windows(EXPORTED.len())
        .enumerate()
        .filter_map(|(offset, bytes)| (bytes == EXPORTED).then_some(offset))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1, "fixture has one Candid debug export");
    let offset = matches[0];
    wasm[offset..offset + EXPORTED.len()].copy_from_slice(HIDDEN);
    assert!(!wasm_exports_candid_pointer(&wasm));
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
