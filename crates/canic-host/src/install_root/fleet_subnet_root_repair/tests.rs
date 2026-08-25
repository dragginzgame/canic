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
        fleet_install_session::{
            CloseFleetInstallSessionRequest, FleetInstallSessionError,
            PlanFleetInstallSessionRequest, close_fleet_install_session,
            plan_fleet_install_session, recover_fleet_install_session_authority,
        },
        fleet_subnet_root_component_registry_preparation::verify_retained_component_registry_preparation,
        fleet_subnet_root_install::verify_pre_repair_root_authority,
        fleet_subnet_root_install_journal::expected_root_authority,
        icp_context::InstallIcpContext,
        operations::require_expected_module_hash,
    },
    protocol_binding::resolve_infrastructure_protocol_binding,
    release_build::{finalize_release_build_from_manifest, plan_release_build},
};
use candid::CandidType;
use canic_core::{
    dto::{
        component_registry::{
            RootComponentRegistryPreparationRequest, RootComponentRegistryStatusResponse,
        },
        fleet_registry::FleetRegistryVersion,
        fleet_subnet_root::FleetSubnetRootAuthority,
        root_store::RootStoreBootstrapRequest,
    },
    ids::{CanisterRole, CanonicalNetworkId, FleetSubnetRootReleaseSet},
    role_contract::{RoleCapabilityKey, derive_protocol_profile_hashes},
};
use pocket_ic::common::rest::{IcpFeatures, IcpFeaturesConfig};
use pocket_ic::{CreateCanisterParams, CreateCanisterPlacement, PocketIc, PocketIcBuilder};
use std::{
    fs,
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
    current.journal.root_artifact.candid_sha256 =
        Sha256::digest(extract_candid_bytes(&artifacts.predecessor_path).expect("retained Candid"))
            .into();
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
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.placement_subnet = SubnetId::from_principal(Principal::from_slice(&[42; 29]));
        },
        |receipt: &mut RetainedRootRepairReceiptV1| receipt.retained_journal_sequence += 1,
        |receipt: &mut RetainedRootRepairReceiptV1| {
            receipt.retained_journal_module_sha256 = [26; 32];
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
        receipt: receipt.clone(),
        needs_publication: true,
        path: repair_receipt_path(&current.path),
    };
    publish_retained_root_repair(&resolved, &session, &current.journal)
        .expect("publish exact repair receipt");
    let replay = resolve_retained_root_repair(&current, &session, None, None)
        .expect("reload exact repair receipt")
        .expect("retained repair receipt");
    assert_eq!(replay.receipt, receipt);
    assert!(!replay.needs_publication);
}

#[test]
fn published_receipt_converges_asset_ready_operation_without_repeating_effects() {
    let (current, session, receipt) = repair_authority_fixture();
    let resolved = ResolvedRetainedRootRepair {
        receipt,
        needs_publication: true,
        path: repair_receipt_path(&current.path),
    };
    super::procedure::write_asset_ready_test_operation(&resolved)
        .expect("retain exact AssetReady interruption point");
    publish_retained_root_repair(&resolved, &session, &current.journal)
        .expect("publish immutable receipt before local operation converges");

    let replay = resolve_retained_root_repair(&current, &session, None, None)
        .expect("reload published repair")
        .expect("published repair receipt");
    assert!(!replay.needs_publication);
    reconcile_published_retained_root_repair(&replay)
        .expect("receipt replay converges only the local operation");
    assert!(
        super::procedure::test_operation_is_adopted(&replay)
            .expect("read terminal repair operation")
    );
    reconcile_published_retained_root_repair(&replay)
        .expect("terminal operation replay is exact and idempotent");

    let mut conflicting_receipt = replay.receipt.clone();
    conflicting_receipt.required_pool_cycles += 1;
    let conflicting = ResolvedRetainedRootRepair {
        receipt: conflicting_receipt,
        needs_publication: false,
        path: replay.path,
    };
    assert!(matches!(
        reconcile_published_retained_root_repair(&conflicting),
        Err(super::procedure::RetainedRootRepairProcedureError::ConflictingAuthority { .. })
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
    fleet_subnet_root: Principal,
    pool_canister: Principal,
    current: ResolvedFleetSubnetRootInstall,
    session: FleetInstallSession,
    icp_context: InstallIcpContext,
    live_predecessor_path: PathBuf,
    successor_path: PathBuf,
    changed_path: PathBuf,
    wrong_candid_path: PathBuf,
    successor_module_sha256: [u8; 32],
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

fn retained_repair_journey_fixture() -> RetainedRepairJourneyFixture {
    let root = crate::test_support::temp_dir("retained-root-repair-pocketic-completion");
    fs::create_dir_all(&root).expect("create retained-repair qualification root");
    let artifacts = write_live_retained_repair_artifacts(&root);
    let session = plan_qualification_session(&root);
    let mut pic = repair_pocket_ic();
    let (icp_executable, controller) = isolated_repair_identity(&root);
    let fleet_subnet_root = pic.create_canister_with_settings(Some(controller), None);
    let root_subnet = pic
        .get_subnet(fleet_subnet_root)
        .expect("repair Root placement Subnet");
    let pool_canister = create_undersized_pool_asset(&pic, root_subnet, fleet_subnet_root);
    let wasm_store = pic.create_canister_on_subnet(None, None, root_subnet);
    let successor_module_sha256: [u8; 32] = Sha256::digest(&artifacts.successor_wasm).into();
    let mut current =
        crate::install_root::fleet_subnet_root_install_journal::tests::planned_repair_fixture(
            &root,
        );
    current.journal.phase = FleetSubnetRootInstallPhase::ComponentRegistryPreparationVerified;
    current.journal.sequence = 28;
    current.journal.fleet_subnet_root = Some(fleet_subnet_root);
    current.journal.wasm_store = Some(wasm_store);
    current.journal.installation_controller = Some(controller);
    current.journal.release_build_id = session.release_build_id;
    current.journal.install_operation_id = session.operation_id;
    current.journal.authority.binding.fleet = session.fleet.clone();
    current.journal.expected_root_module_hash = Sha256::digest(&artifacts.predecessor_wasm).into();
    bind_repair_stub_artifact(&root, &mut current, &artifacts);
    let (retained_component_registry, observed_component_registry) =
        component_registry_proof(&current, fleet_subnet_root);
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
    let root_authority =
        expected_root_authority(&current.journal).expect("compile exact retained Root authority");
    let pool_cycles = pic.cycle_balance(pool_canister);
    pic.install_canister(
        fleet_subnet_root,
        artifacts.live_predecessor_wasm.clone(),
        candid::encode_one(RepairStubInit {
            authority: root_authority,
            pool_canister,
            pool_cycles,
            component_registry: observed_component_registry,
        })
        .expect("encode retained repair Root fixture authority"),
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
        fleet_subnet_root,
        pool_canister,
        current,
        session,
        icp_context,
        live_predecessor_path: artifacts.live_predecessor_path,
        successor_path: artifacts.successor_path,
        changed_path: artifacts.changed_path,
        wrong_candid_path: artifacts.wrong_candid_path,
        successor_module_sha256,
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
    let compatible_candid = "service : { ping: () -> (); }\n";
    let predecessor_wasm = minimal_canister_wasm(compatible_candid, "predecessor");
    let live_predecessor_wasm = minimal_canister_wasm(compatible_candid, "live-predecessor");
    let successor_wasm = minimal_canister_wasm(compatible_candid, "successor");
    let changed_wasm = minimal_canister_wasm(compatible_candid, "changed-successor");
    write_retained_repair_artifact_set(
        root,
        predecessor_wasm,
        live_predecessor_wasm,
        successor_wasm,
        changed_wasm,
    )
}

fn write_live_retained_repair_artifacts(root: &Path) -> RetainedRepairArtifacts {
    let base = build_repair_stub_wasm();
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
    )
}

fn write_retained_repair_artifact_set(
    root: &Path,
    predecessor_wasm: Vec<u8>,
    live_predecessor_wasm: Vec<u8>,
    successor_wasm: Vec<u8>,
    changed_wasm: Vec<u8>,
) -> RetainedRepairArtifacts {
    let wrong_candid_wasm =
        minimal_canister_wasm("service : { ping: (nat) -> (); }\n", "wrong-candid");
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
}

fn bind_repair_stub_artifact(
    root: &Path,
    current: &mut ResolvedFleetSubnetRootInstall,
    artifacts: &RetainedRepairArtifacts,
) {
    let candid = extract_candid_bytes(&artifacts.predecessor_path)
        .expect("extract retained repair stub Candid");
    let role = CanisterRole::from("root");
    let capabilities = std::collections::BTreeSet::from([RoleCapabilityKey::Root]);
    let release_identity = current
        .journal
        .root_artifact
        .protocol_release_identity
        .clone();
    let profile = derive_protocol_profile_hashes(&release_identity, &role, &capabilities, &candid);
    let artifact_relative_path = "retained-repair/root.wasm".to_string();
    let artifact_path = root.join(&artifact_relative_path);
    fs::create_dir_all(artifact_path.parent().expect("repair artifact parent"))
        .expect("create repair artifact parent");
    fs::write(&artifact_path, &artifacts.predecessor_wasm)
        .expect("write retained original Root artifact");
    fs::write(artifact_path.with_extension("did"), &candid)
        .expect("write exact retained Root Candid sidecar");
    current.journal.root_artifact.protocol_role = role;
    current.journal.root_artifact.protocol_capabilities = capabilities;
    current.journal.root_artifact.wasm_relative_path = artifact_relative_path;
    current.journal.root_artifact.wasm_size_bytes =
        u64::try_from(artifacts.predecessor_wasm.len()).expect("Root Wasm size");
    current.journal.root_artifact.wasm_sha256_hex =
        encode_hex(&Sha256::digest(&artifacts.predecessor_wasm));
    current.journal.root_artifact.candid_sha256 = profile.candid_sha256;
    current.journal.root_artifact.protocol_profile_digest = profile.protocol_profile_digest;
}

fn component_registry_proof(
    current: &ResolvedFleetSubnetRootInstall,
    fleet_subnet_root: Principal,
) -> (
    RootComponentRegistryStatusResponse,
    RootComponentRegistryStatusResponse,
) {
    let retained = RootComponentRegistryStatusResponse {
        fleet_subnet_root,
        prepared_against_registry: FleetRegistryVersion {
            authority: current.journal.authority.clone(),
            revision: 2,
            content_hash: [0x42; 32],
        },
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
    (retained, observed)
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

fn execute_exact_repair_and_reject_conflicts(
    fixture: &RetainedRepairJourneyFixture,
) -> RetainedRootRepairReceiptV1 {
    let (repair, root_binding) = resolve_repair_for_execution(fixture);
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
        &fixture.successor_path,
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
    verify_retained_component_registry_preparation(&fixture.icp_context, &fixture.current.journal)
        .expect("verify advanced Component Registry through status-like replay");
    assert_eq!(
        fixture
            .pic
            .canister_status(fixture.fleet_subnet_root, Some(fixture.controller))
            .expect("read final repaired Root status")
            .module_hash,
        Some(fixture.successor_module_sha256.to_vec())
    );
    publish_retained_root_repair(&repair, &fixture.session, &fixture.current.journal)
        .expect("publish exact repair receipt after live PocketIC verification");
    record_retained_root_repair_adopted(&repair, operation)
        .expect("make the exact repair operation terminal");
    assert!(
        super::procedure::test_operation_is_adopted(&repair)
            .expect("read terminal production repair operation")
    );
    assert_repair_replay_has_no_effect(fixture, &repair, operator_before, asset_before);

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
    repair.receipt
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
        repair.receipt.successor_module_hash(),
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
        &repair.receipt,
    )
    .expect("verify live predecessor controller, module and Fleet authority");
    (repair, root_binding)
}

fn assert_repair_replay_has_no_effect(
    fixture: &RetainedRepairJourneyFixture,
    repair: &ResolvedRetainedRootRepair,
    operator_before: u128,
    asset_before: u128,
) {
    let operator_after = fixture
        .icp_context
        .cli()
        .identity_cycles_balance()
        .expect("observe post-repair operator cycles");
    let asset_after = fixture.pic.cycle_balance(fixture.pool_canister);
    assert!(asset_after >= 5_000_000_000_000);
    assert_eq!(
        operator_before - operator_after,
        (asset_after - asset_before) + RETAINED_ROOT_REPAIR_TOP_UP_FEE_CYCLES
    );
    reconcile_published_retained_root_repair(repair)
        .expect("terminal receipt replay remains effect-free");
    assert_eq!(
        fixture.pic.cycle_balance(fixture.pool_canister),
        asset_after
    );
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
        repair_operation_id: repair_operation_id(session, journal, &transition)
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
        pool_canister,
        installation_controller: journal.installation_controller.expect("controller"),
        retained_journal_phase: journal.phase,
        retained_journal_sequence: journal.sequence,
        retained_journal_module_sha256: journal.expected_root_module_hash,
        upgrade_predecessor_module_sha256,
        upgrade_predecessor_wasm_size_bytes: 1,
        successor_module_sha256,
        successor_wasm_size_bytes: 1,
        retained_journal_candid_sha256: journal.root_artifact.candid_sha256,
        upgrade_predecessor_candid_sha256: journal.root_artifact.candid_sha256,
        successor_candid_sha256: journal.root_artifact.candid_sha256,
        required_pool_cycles: 5_000_000_000_000,
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
