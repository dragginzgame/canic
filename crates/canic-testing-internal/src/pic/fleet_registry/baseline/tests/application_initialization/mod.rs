//! Actual Root/Store/Coordinator installation with opaque, target-bound app bytes.

use super::*;
use canic::dto::component_registry::{
    ComponentApplicationInitialization, RootComponentInitializationRequest,
};

#[derive(CandidType)]
enum Command {
    BindComponentInitialization(RootComponentInitializationRequest),
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "ordered production installation and interruption journey"
)]
pub(super) fn production_initialization_recovers_exact_target_bytes() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"))
        .expect("discover Canic test workspace through Cargo");
    let config_path =
        workspace.join("canisters/test/managed_lifecycle_probe/canic.initialization.toml");
    let profile = crate::pic::artifacts::CanicWasmBuildProfile::Fast;
    let root_wasm = crate::pic::artifacts::build_generated_fleet_wasm(
        &workspace,
        &config_path,
        "root",
        profile,
    );
    let coordinator_wasm = crate::pic::artifacts::build_generated_fleet_wasm(
        &workspace,
        &config_path,
        "fleet_coordinator",
        profile,
    );
    let target_dir = test_target_dir(&workspace, "application-initialization");
    let modules = crate::pic::artifacts::build_internal_test_wasm_canisters_with_env(
        &workspace,
        &target_dir,
        &["managed_lifecycle_probe"],
        profile,
        &[(
            canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
            config_path.to_str().unwrap(),
        )],
    );
    let probe_wasm = modules.wasm("managed_lifecycle_probe");
    let roles = BTreeMap::from([(CanisterRole::new("test"), probe_wasm.clone())]);
    let pic = build_pic();
    let coordinator = pic.create_canister();
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
    let fixture = install_bootstrapped_root_with_config_and_pool_setup(
        &pic,
        root_wasm.clone(),
        coordinator,
        build_root_store_fixture_with_config(&config_path, &roles),
        BootstrappedRootPlacement {
            canister_pool_maximum_size: None,
            canister_pool_minimum_size: None,
            canister_pool_cycles: None,
            coordinator_subnet: None,
            existing_root: None,
            existing_wasm_store: None,
            root_subnet: None,
            component_admission_limits: None,
            fleet_id: None,
            funding: None,
            coordinator_root_funding: None,
        },
        &config_path,
        create_prepaid_pool_assets,
    );
    let operation = [0xd4; 32];
    begin_fixture_fresh_component_provisioning_with_config(
        &pic,
        coordinator,
        coordinator_wasm,
        &fixture,
        operation,
        &config_path,
    );
    let pending = await_root_provisioning(&pic, fixture.root_id, operation, |status| {
        status.claimed_component_count == 1 && status.installed_component_count == 0
    });
    assert_eq!(pending.operation_id, operation);
    let claim = root_pool_status(&pic, fixture.root_id)
        .entries
        .into_iter()
        .find_map(|entry| {
            if let CanisterPoolAssetStatus::Claimed { claim }
            | CanisterPoolAssetStatus::Workload { claim } = entry.status
            {
                Some((entry.canister_id, claim.operation_id))
            } else {
                None
            }
        })
        .expect("one allocated Component held for initialization");
    let target = claim.0;
    let member_operation = claim.1;
    let initialization = ComponentApplicationInitialization {
        target_canister: target,
        arguments: vec![255; canic::dto::component_registry::MAX_COMPONENT_APPLICATION_INIT_BYTES],
    };
    let request = RootComponentInitializationRequest {
        operation_id: member_operation,
        initialization,
    };
    let before = allocation(&pic, fixture.root_id, member_operation);
    assert!(before.application_initialization.is_none());
    let status_refused: Result<RootStatusResponseFragment, Error> = pic
        .query_candid_as(
            fixture.root_id,
            target,
            canic::protocol::CANIC_ROOT_OPERATION_STATUS,
            (RootStatusRequestFragment::Operation(
                OperationStatusRequest {
                    operation_id: member_operation,
                },
            ),),
        )
        .unwrap();
    assert_eq!(
        status_refused
            .err()
            .expect("controller-only allocation status")
            .code(),
        canic::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
    );
    let mut wrong = request.clone();
    wrong.initialization.target_canister = coordinator;
    let rejected: Result<RootCommandResponseFragment, Error> = pic
        .update_candid_as(
            fixture.root_id,
            Principal::anonymous(),
            canic::protocol::CANIC_ROOT_COMMAND,
            (Command::BindComponentInitialization(wrong),),
        )
        .unwrap();
    assert_eq!(
        rejected.err().expect("target mismatch refusal").code(),
        canic::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    let refused: Result<RootCommandResponseFragment, Error> = pic
        .update_candid_as(
            fixture.root_id,
            target,
            canic::protocol::CANIC_ROOT_COMMAND,
            (Command::BindComponentInitialization(request.clone()),),
        )
        .unwrap();
    assert_eq!(
        refused.err().expect("controller refusal").code(),
        canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code()
    );
    assert_eq!(allocation(&pic, fixture.root_id, member_operation), before);
    // Replace the heap while the production worker is still waiting for init.
    pic.stop_canister(fixture.root_id, None).unwrap();
    pic.upgrade_canister(
        fixture.root_id,
        root_wasm.clone(),
        crate::pic::upgrade_args(),
        None,
    )
    .unwrap();
    pic.start_canister(fixture.root_id, None).unwrap();
    assert_eq!(allocation(&pic, fixture.root_id, member_operation), before);
    // Discard this ingress reply: reconcile the durable binding, then replay it.
    let bytes = canic_host::component_initialization::encode_command(&request).unwrap();
    let _lost_reply = pic
        .submit_call(
            fixture.root_id,
            Principal::anonymous(),
            canic::protocol::CANIC_ROOT_COMMAND,
            bytes,
        )
        .unwrap();
    for _ in 0..120 {
        if allocation(&pic, fixture.root_id, member_operation)
            .application_initialization
            .is_some()
        {
            break;
        }
        pic.tick();
    }
    let bound = allocation(&pic, fixture.root_id, member_operation);
    assert_eq!(
        bound.application_initialization,
        Some(request.initialization.clone())
    );
    let replay: Result<RootCommandResponseFragment, Error> = pic
        .update_candid_as(
            fixture.root_id,
            Principal::anonymous(),
            canic::protocol::CANIC_ROOT_COMMAND,
            (Command::BindComponentInitialization(request.clone()),),
        )
        .unwrap();
    replay.unwrap();
    restart_root_during_installation(
        &pic,
        fixture.root_id,
        target,
        member_operation,
        operation,
        root_wasm,
    );
    await_root_provisioning(&pic, fixture.root_id, operation, |status| {
        status.root_runtime_active
    });
    let delivered: Result<Vec<u8>, Error> = pic
        .query_candid_as(
            target,
            Principal::anonymous(),
            "managed_initialization_bytes",
            (),
        )
        .unwrap();
    assert_eq!(delivered.unwrap(), request.initialization.arguments);
    let installation_version = pic
        .canister_status(target, Some(fixture.root_id))
        .unwrap()
        .version;
    let replay: Result<RootCommandResponseFragment, Error> = pic
        .update_candid_as(
            fixture.root_id,
            Principal::anonymous(),
            canic::protocol::CANIC_ROOT_COMMAND,
            (Command::BindComponentInitialization(request.clone()),),
        )
        .unwrap();
    replay.unwrap();
    assert_eq!(
        pic.canister_status(target, Some(fixture.root_id))
            .unwrap()
            .version,
        installation_version
    );
    pic.upgrade_canister(
        target,
        probe_wasm,
        crate::pic::upgrade_args(),
        Some(fixture.root_id),
    )
    .unwrap();
    let restored: Result<Vec<u8>, Error> = pic
        .query_candid_as(
            target,
            Principal::anonymous(),
            "managed_initialization_bytes",
            (),
        )
        .unwrap();
    assert_eq!(restored.unwrap(), request.initialization.arguments);
}

fn allocation(
    pic: &PocketIc,
    root: Principal,
    operation_id: [u8; 32],
) -> canic::dto::component_registry::RootComponentAllocationResponse {
    let RootStatusResponseFragment::Operation(RootOperationStatusResponse::ProvisionComponent(
        status,
    )) = root_status(
        pic,
        root,
        RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
    )
    .unwrap()
    else {
        panic!("exact Component allocation status")
    };
    status.allocation
}

fn restart_root_during_installation(
    pic: &PocketIc,
    root: Principal,
    target: Principal,
    operation_id: [u8; 32],
    provisioning_operation_id: [u8; 32],
    root_wasm: Vec<u8>,
) {
    let mut phases = Vec::new();
    for _ in 0..240 {
        let phase = allocation(pic, root, operation_id).phase;
        if !phases.contains(&phase) {
            phases.push(phase);
        }
        if phase == RootComponentAllocationPhase::InstallIntent {
            // The IC quiesces pending callbacks before upgrade. This proves an
            // actual restart during installation, not a dropped management reply.
            pic.upgrade_canister(root, root_wasm, crate::pic::upgrade_args(), None)
                .unwrap();
            assert_eq!(
                allocation(pic, root, operation_id).phase,
                RootComponentAllocationPhase::Committed
            );
            return;
        }
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
    }
    panic!(
        "install intent was not observed before Root restart: phases={phases:?}, query={:?}, provisioning={:?}",
        pic.query_call(
            target,
            Principal::anonymous(),
            "managed_initialization_bytes",
            candid::encode_args(()).unwrap()
        )
        .map(|bytes| bytes.len()),
        observed_root_provisioning(pic, root, provisioning_operation_id)
            .map(|status| (status.phase, status.last_failure)),
    );
}
