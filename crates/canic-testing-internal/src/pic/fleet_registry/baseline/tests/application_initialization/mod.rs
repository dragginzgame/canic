//! Actual Root/Store/Coordinator installation with opaque, target-bound app bytes.

mod blob;

use super::*;
use canic::dto::component_registry::{
    ComponentApplicationInitialization, RootComponentInitializationRequest,
};

use canic_contracts::protocol::component_initialization as initialization_wire;

pub(super) use blob::production_blob_initialization_recovers_target_binding;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "ordered production installation and interruption journey"
)]
pub(super) fn production_initialization_recovers_exact_target_bytes() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace =
        crate::workspace::discover().expect("discover Canic test workspace through Cargo");
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
    let target_dir = workspace.join("target").join("application-initialization");
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
    let status_refused = crate::pic::canic::root_component_allocation(
        &pic,
        fixture.root_id,
        target,
        member_operation,
    );
    assert_eq!(
        status_refused
            .expect_err("controller-only allocation status")
            .code(),
        canic::diagnostics::codes::AUTHORITY_UNAUTHORIZED.raw_code()
    );
    assert_malformed_binding_frames(&pic, fixture.root_id, member_operation, &request);
    let mut wrong = request.clone();
    wrong.initialization.target_canister = coordinator;
    let rejected = bind_initialization(&pic, fixture.root_id, Principal::anonymous(), wrong);
    assert_eq!(
        rejected.expect_err("target mismatch refusal").code(),
        canic::diagnostics::codes::STATE_CONFLICT.raw_code()
    );
    let refused = bind_initialization(&pic, fixture.root_id, target, request.clone());
    assert_eq!(
        refused.expect_err("controller refusal").code(),
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
            initialization_wire::METHOD,
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
    let replay = bind_initialization(
        &pic,
        fixture.root_id,
        Principal::anonymous(),
        request.clone(),
    );
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
    let delivered: Result<Vec<u8>, Error> = ic_testkit::pic::query_wire(
        &pic,
        target,
        Principal::anonymous(),
        crate::pic::application_probe::MANAGED_INITIALIZATION,
        &(),
    )
    .unwrap();
    assert_eq!(delivered.unwrap(), request.initialization.arguments);
    let installation_version = pic
        .canister_status(target, Some(fixture.root_id))
        .unwrap()
        .version;
    let replay = bind_initialization(
        &pic,
        fixture.root_id,
        Principal::anonymous(),
        request.clone(),
    );
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
    let restored: Result<Vec<u8>, Error> = ic_testkit::pic::query_wire(
        &pic,
        target,
        Principal::anonymous(),
        crate::pic::application_probe::MANAGED_INITIALIZATION,
        &(),
    )
    .unwrap();
    assert_eq!(restored.unwrap(), request.initialization.arguments);
}

fn allocation(
    pic: &PocketIc,
    root: Principal,
    operation_id: [u8; 32],
) -> canic::dto::component_registry::RootComponentAllocationResponse {
    crate::pic::canic::root_component_allocation(pic, root, Principal::anonymous(), operation_id)
        .expect("native Component allocation status")
        .allocation
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
        ic_testkit::pic::query_wire::<_, Result<Vec<u8>, Error>>(
            pic,
            target,
            Principal::anonymous(),
            crate::pic::application_probe::MANAGED_INITIALIZATION,
            &(),
        )
        .map(|reply| reply.map(|bytes| bytes.len())),
        observed_root_provisioning(pic, root, provisioning_operation_id)
            .map(|status| (status.phase, status.last_failure)),
    );
}

fn bind_initialization(
    pic: &PocketIc,
    root: Principal,
    caller: Principal,
    request: RootComponentInitializationRequest,
) -> Result<canic::dto::role::OperationReceipt, Error> {
    let bytes = canic_host::component_initialization::encode_command(&request).unwrap();
    let response = ic_testkit::pic::try_pocket_ic_operation(|| {
        pic.update_call(root, caller, initialization_wire::METHOD, bytes)
    })
    .unwrap()
    .unwrap();
    ic_wire::native::decode_response(
        initialization_wire::METHOD_ID,
        &response,
        initialization_wire::RESPONSE_LIMITS,
    )
    .unwrap()
}

fn assert_malformed_binding_frames(
    pic: &PocketIc,
    root: Principal,
    member_operation: [u8; 32],
    request: &RootComponentInitializationRequest,
) {
    let before = allocation(pic, root, member_operation);
    let native =
        canic_contracts::dto::component_initialization::RootComponentInitializationRequest::from(
            request.clone(),
        );
    let frame = canic_host::component_initialization::encode_command(request).unwrap();
    let mut trailing = frame.clone();
    trailing.push(0);
    let mut truncated = frame;
    truncated.pop();
    let wrong_method = ic_wire::native::encode_request(
        initialization_wire::METHOD_ID + 1,
        &native,
        initialization_wire::REQUEST_LIMITS,
    )
    .unwrap();
    let wrong_direction = ic_wire::native::encode_response(
        initialization_wire::METHOD_ID,
        &native,
        initialization_wire::REQUEST_LIMITS,
    )
    .unwrap();
    for bytes in [
        wrong_method,
        wrong_direction,
        trailing,
        truncated,
        vec![0; initialization_wire::REQUEST_LIMITS.bytes + 1],
    ] {
        let result = ic_testkit::pic::try_pocket_ic_operation(|| {
            pic.update_call(
                root,
                Principal::anonymous(),
                initialization_wire::METHOD,
                bytes,
            )
        })
        .unwrap();
        assert!(result.is_err());
        assert_eq!(allocation(pic, root, member_operation), before);
    }
}
