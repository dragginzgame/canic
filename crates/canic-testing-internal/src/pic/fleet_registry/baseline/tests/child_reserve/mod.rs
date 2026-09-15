//! Qualify a retained low-native-reserve child claim and its same-operation recovery.

use super::*;
use canic::dto::component_registry::{
    ComponentRegistryPartitionRequest, RootComponentChildAllocationRequest,
};
use canic_control_plane::dto::root::RootComponentChildOperationStatus;

#[derive(CandidType)]
enum Command {
    ProvisionChild(RootComponentChildAllocationRequest),
    SetCyclesFunding(canic::dto::state::SetCyclesFundingRequest),
}

#[derive(CandidType, Deserialize)]
enum Response {
    OperationAccepted(canic::dto::role::OperationReceipt),
    SetCyclesFunding(canic::dto::state::FleetStateCommandResult<bool>),
}

#[test]
pub(super) fn initial_child_failure_reaches_coordinator_and_recovers_same_claim() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
    let config_path = initial_shard_root_canister_config_path(&workspace);
    let config = AppConfigSnapshot::load(&config_path).unwrap();
    let mut pic = build_management_pic();
    let coordinator = pic.create_canister();
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
    let fixture = install_fixture_root(
        &pic,
        coordinator,
        &config_path,
        &[],
        crate::pic::fleet_registry::build::build_child_reserve_root_wasm(),
        build_initial_shard_component_wasms(),
    );
    let root = fixture.root_id;
    let (version, sync) = join_and_synchronize_root(&pic, coordinator, &fixture);
    activate_registry_and_prepare_component_registry(&pic, coordinator, &fixture, version, sync);
    let CoordinatorRegistryResponse::Registry(registry) =
        coordinator_status(&pic, coordinator, CoordinatorRegistryRequest::Registry).unwrap();
    let operation_id = [0x93; 32];
    let plan = fixture_fresh_component_plan(config.model(), &registry, operation_id);
    coordinator_command(
        &pic,
        coordinator,
        CoordinatorCommand::ProvisionComponents(plan.request.clone()),
    )
    .unwrap();
    wait_for_parent_commit(&pic, root, operation_id);
    assert_balance_control_requires_controller(&pic, root);
    let burned: u128 = pic.update_candid_as_or_panic(
        root,
        Principal::anonymous(),
        "audit_recovery_balance",
        (800_000_000_000_u128,),
    );
    assert!(burned > 0);
    let origin = (0..240).find_map(|_| {
        let status = provisioning_status(&pic, coordinator, operation_id);
        let origin = status.pending_root_failure.and_then(|failure| failure.origin)
            .filter(|origin| origin.stage == canic::dto::component_provisioning::ProvisioningFailureStage::ComponentChildAllocation);
        if origin.is_none() {
            pic.advance_time(Duration::from_secs(1));
            pic.tick();
        }
        origin
    }).expect("Coordinator must retain the initial child's exact failure");
    assert_eq!(origin.target, root);
    assert_eq!(
        origin.diagnostic_code,
        canic_core::diagnostics::codes::DEPLOYMENT_CYCLE_RESERVE_REQUIRED
            .raw_code()
            .raw()
    );
    let child = operation(&pic, root, origin.operation_id);
    assert_eq!(
        child.allocation.phase,
        RootComponentAllocationPhase::Reserved
    );
    assert!(child.allocation.creation.is_none());
    assert!(child.allocation.installation.is_none());
    let claim = root_pool_status(&pic, root).entries.into_iter().find(|entry| {
        matches!(&entry.status, CanisterPoolAssetStatus::Claimed { claim } if claim.operation_id == origin.operation_id)
    }).expect("the same failed initial claim remains reserved");
    assert_host_origin(&mut pic, coordinator, &plan, &origin);
    pic.add_cycles(root, 5_000_000_000_000);
    let mut complete = false;
    for _ in 0..240 {
        let status = provisioning_status(&pic, coordinator, operation_id);
        if status.runtimes_activated_at_ns.is_some() {
            assert!(status.pending_root_failure.is_none());
            complete = true;
            break;
        }
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
    }
    assert!(
        complete,
        "bootstrap completes after replenishing its retained Root"
    );
    let recovered = operation(&pic, root, origin.operation_id);
    assert!(recovered.allocation.last_failure.is_none());
    assert_eq!(
        recovered.allocation.creation.unwrap().canister,
        Some(claim.canister_id)
    );
    assert_eq!(
        fixture_readiness(&pic, root, claim.canister_id).status,
        canic::dto::runtime::ReadinessStatus::Ready
    );
}

/// Use the maintained host observer with a real selected operator and protected IC status.
fn assert_host_origin(
    pic: &mut PocketIc,
    coordinator: Principal,
    plan: &CompiledCurrentComponentProvisioning,
    expected: &canic::dto::component_provisioning::ProvisioningFailureOrigin,
) {
    let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
    let directory = literal_zero_adapter_root(&workspace).join("child-origin");
    let _cleanup = TestDirectoryCleanup(directory.clone());
    let (wrapper, operator, _) = prepare_isolated_icp(&directory);
    pic.set_controllers(coordinator, None, vec![Principal::anonymous(), operator])
        .unwrap();
    // Observation needs only its exact Coordinator target. No plan, paid effect,
    // creation inventory or funding authority is manufactured by this fixture.
    let desired: DesiredFleet = serde_json::from_value(serde_json::json!({
        "canisters": [{
            "controllers": [Principal::anonymous().to_text(), operator.to_text()],
            "initial_cycles": "0", "kind": "coordinator", "minimum_cycles": "0",
            "name": "coordinator", "presence": "present", "principal": coordinator.to_text(),
            "replace": false, "subnet": pic.get_subnet(coordinator).unwrap().to_text()
        }],
        "cycles_ledger": Principal::management_canister().to_text(),
        "environment": "local", "fleet": "child-origin", "ledger_fee_cycles": "0",
        "management_creation_fee_cycles": "0", "material_cycle_threshold": "0",
        "maximum_observation_burn_cycles": "0", "maximum_stalled_observations": 2,
        "maximum_update_burn_cycles": "0", "operator": operator.to_text(),
        "schema_version": 1, "treasury": "coordinator"
    }))
    .unwrap();
    let paths =
        canic_host::fleet_ensure::ops::EnsurePaths::under(&directory, "local", "child-origin");
    let state = canic_host::fleet_ensure::ops::read_state(&paths, "child-origin").unwrap();
    let candid = workspace.join("crates/canic/candid/fleet_coordinator.did");
    let action = EnsureAction::FleetProtocol {
        action: Box::new(CurrentFleetProtocolAction::ProvisionComponents {
            plan_hash: plan.plan_hash,
            request: plan.request.clone(),
        }),
        candid: candid.to_str().unwrap().into(),
        candid_sha256: hex_bytes(wasm_hash(&std::fs::read(candid).unwrap())),
        maximum_execution_burn_cycles: 0,
        name: "component-provisioning".into(),
        principal: coordinator.to_text(),
    };
    let mut effect = fixture_effect_intent(&action);
    effect.state = EffectState::Issued;
    let url = pic.make_live(None);
    let mut platform = IcpEnsurePlatform::new(desired, wrapper.to_str().unwrap(), &directory)
        .with_local_replica(LocalReplicaTarget {
            environment: "local".into(),
            root_key: hex_bytes(pic.root_key().unwrap()),
            url: url.to_string(),
        });
    let observed = platform.observe_effect(
        &hex_bytes(plan.request.operation_id),
        &action,
        &effect,
        &state,
    );
    pic.stop_live();
    let observed = observed.unwrap();
    assert!(!observed.applied);
    let origin = observed.provisioning_failure.unwrap().origin.unwrap();
    assert_eq!(origin.stage, expected.stage);
    assert_eq!(origin.target, expected.target);
    assert_eq!(origin.operation_id, expected.operation_id);
    assert_eq!(origin.diagnostic_code, expected.diagnostic_code);
    assert_eq!(origin.retry_category, expected.retry_category);
}

fn assert_balance_control_requires_controller(pic: &PocketIc, root: Principal) {
    let denied = pic
        .update_call(
            root,
            Principal::from_slice(&[0x95; 29]),
            "audit_recovery_balance",
            candid::encode_args((0_u128,)).unwrap(),
        )
        .expect_err("a non-controller cannot burn the audit Root's balance");
    assert_eq!(
        denied.reject_code,
        ic_testkit::pocket_ic::RejectCode::CanisterReject
    );
}

fn wait_for_parent_commit(pic: &PocketIc, root: Principal, operation_id: [u8; 32]) {
    for _ in 0..240 {
        if let Ok(RootStatusResponseFragment::Operation(
            RootOperationStatusResponse::ProvisionComponents(status),
        )) = root_status(
            pic,
            root,
            RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
        ) && status.component_count > 0
            && status.registry_committed_component_count == status.component_count
        {
            assert!(!status.root_runtime_active);
            return;
        }
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
    }
    panic!("top-level Component must commit before lowering the bootstrap reserve");
}

fn provisioning_status(
    pic: &PocketIc,
    coordinator: Principal,
    operation_id: [u8; 32],
) -> canic::dto::component_provisioning::FleetComponentProvisioningStatusResponse {
    let CoordinatorOperationReadResponse::Operation(
        CoordinatorOperationStatusResponse::ComponentProvisioning(status),
    ) = coordinator_status(
        pic,
        coordinator,
        CoordinatorOperationReadRequest::Operation(OperationStatusRequest { operation_id }),
    )
    .unwrap()
    else {
        panic!("correlated provisioning operation")
    };
    status
}

#[test]
pub(super) fn low_native_reserve_retains_child_failure_and_recovers_same_claim() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
    let config_path = initial_shard_root_canister_config_path(&workspace);
    let config = AppConfigSnapshot::load(&config_path).unwrap();
    let pic = build_pic();
    let coordinator = pic.create_canister();
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
    let fixture = install_fixture_root(
        &pic,
        coordinator,
        &config_path,
        &[],
        crate::pic::fleet_registry::build::build_child_reserve_root_wasm(),
        build_initial_shard_component_wasms(),
    );
    super::state_cascade::activate_components(&pic, coordinator, &fixture, &config);
    let root = fixture.root_id;
    let parent = root_pool_status(&pic, root)
        .entries
        .into_iter()
        .filter(|entry| matches!(entry.status, CanisterPoolAssetStatus::Workload { .. }))
        .find_map(
            |entry| match managed_binding_status(&pic, root, entry.canister_id) {
                ManagedCanisterBinding::Component(binding)
                    if binding.role.as_str() == "user_hub" =>
                {
                    Some(binding)
                }
                _ => None,
            },
        )
        .expect("active parent Hub");
    let RootStatusResponseFragment::ComponentRegistryPartition(partition) = root_status(
        &pic,
        root,
        RootStatusRequestFragment::ComponentRegistryPartition(ComponentRegistryPartitionRequest {
            component: parent.component,
        }),
    )
    .unwrap() else {
        panic!("partition response")
    };
    prepare_spare(&pic, root);
    pause_and_reduce_reserve(&pic, root);
    let request = RootComponentChildAllocationRequest {
        operation_id: [0x92; 32],
        component: parent.component,
        expected_registry: partition.head,
        child_role: CanisterRole::new("user_shard"),
        application_init_args: None,
    };
    provision(&pic, root, parent.canister_id, &request);
    let blocked = wait_for(&pic, root, request.operation_id, |status| {
        status.allocation.last_failure.is_some()
    });
    let failure = blocked.allocation.last_failure.unwrap();
    assert_eq!(
        failure.diagnostic_code,
        canic_core::diagnostics::codes::DEPLOYMENT_CYCLE_RESERVE_REQUIRED
            .raw_code()
            .raw()
    );
    assert_eq!(
        blocked.allocation.phase,
        RootComponentAllocationPhase::Reserved
    );
    assert!(blocked.allocation.creation.is_none());
    assert!(blocked.allocation.installation.is_none());
    let claim = root_pool_status(&pic, root).entries.into_iter().find(|entry| {
        matches!(&entry.status, CanisterPoolAssetStatus::Claimed { claim } if claim.operation_id == request.operation_id)
    }).expect("retained pool claim");
    pic.add_cycles(root, 5_000_000_000_000);
    let recovered = wait_for(&pic, root, request.operation_id, |status| {
        status.allocation.last_failure.is_none()
            && status.allocation.phase == RootComponentAllocationPhase::Committed
            && status
                .allocation
                .creation
                .as_ref()
                .and_then(|creation| creation.canister)
                .is_some_and(|child| {
                    fixture_readiness(&pic, root, child).status
                        == canic::dto::runtime::ReadinessStatus::Ready
                })
    });
    assert_eq!(
        recovered.allocation.creation.as_ref().unwrap().canister,
        Some(claim.canister_id)
    );
    provision(&pic, root, parent.canister_id, &request);
    assert_eq!(operation(&pic, root, request.operation_id), recovered);
    let matches = root_pool_status(&pic, root).entries.into_iter().filter(|entry| {
        matches!(&entry.status, CanisterPoolAssetStatus::Workload { claim } if claim.operation_id == request.operation_id)
    }).collect::<Vec<_>>();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].canister_id, claim.canister_id);
}

fn prepare_spare(pic: &PocketIc, root: Principal) {
    let spare = root_pool_status(pic, root)
        .entries
        .into_iter()
        .find(|entry| {
            matches!(
                entry.status,
                CanisterPoolAssetStatus::Ready | CanisterPoolAssetStatus::PendingReset
            )
        })
        .expect("existing spare pool asset");
    pic.add_cycles(spare.canister_id, 20_000_000_000_000);
    for _ in 0..30 {
        let response = root_command(
            pic,
            root,
            RootCommandFragment::ImportPoolCanister(PoolCanisterRequest {
                canister_id: spare.canister_id,
            }),
        );
        match response {
            Ok(RootCommandResponseFragment::ImportPoolCanister(PoolImportResponse::Imported {
                ..
            })) => {}
            Err(error) => assert_eq!(
                error.code(),
                canic_core::diagnostics::codes::STATE_CONFLICT.raw_code()
            ),
            _ => panic!("correlated pool import"),
        }
        if root_pool_status(pic, root).entries.iter().any(|entry| {
            entry.canister_id == spare.canister_id
                && matches!(entry.status, CanisterPoolAssetStatus::Ready)
                && entry.cycles.to_u128() >= 20_000_000_000_000
        }) {
            return;
        }
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
    }
    panic!("funded spare must be Ready before lowering the Root reserve");
}

fn pause_and_reduce_reserve(pic: &PocketIc, root: Principal) {
    let paused: Result<Response, Error> = pic.update_candid_as_or_panic(
        root,
        Principal::anonymous(),
        canic::protocol::CANIC_ROOT_COMMAND,
        (Command::SetCyclesFunding(
            canic::dto::state::SetCyclesFundingRequest { enabled: false },
        ),),
    );
    let Response::SetCyclesFunding(paused) = paused.unwrap() else {
        panic!("funding response")
    };
    assert_eq!(paused.propagation.unconfirmed_targets, 0);
    assert!(paused.reconciliation_error.is_none());
    let burned: u128 = pic.update_candid_as_or_panic(
        root,
        Principal::anonymous(),
        "audit_recovery_balance",
        (800_000_000_000_u128,),
    );
    assert!(burned > 0);
    assert!(pic.cycle_balance(root) < 1_000_000_000_000);
}

fn provision(
    pic: &PocketIc,
    root: Principal,
    parent: Principal,
    request: &RootComponentChildAllocationRequest,
) {
    let response: Result<Response, Error> = pic.update_candid_as_or_panic(
        root,
        parent,
        canic::protocol::CANIC_ROOT_COMMAND,
        (Command::ProvisionChild(request.clone()),),
    );
    let Response::OperationAccepted(receipt) = response.unwrap() else {
        panic!("accepted child")
    };
    assert_eq!(receipt.operation_id, request.operation_id);
}

fn operation(
    pic: &PocketIc,
    root: Principal,
    operation_id: [u8; 32],
) -> RootComponentChildOperationStatus {
    let RootStatusResponseFragment::Operation(RootOperationStatusResponse::ProvisionChild(status)) =
        root_status(
            pic,
            root,
            RootStatusRequestFragment::Operation(OperationStatusRequest { operation_id }),
        )
        .unwrap()
    else {
        panic!("child operation")
    };
    status
}

fn wait_for(
    pic: &PocketIc,
    root: Principal,
    operation_id: [u8; 32],
    predicate: impl Fn(&RootComponentChildOperationStatus) -> bool,
) -> RootComponentChildOperationStatus {
    for _ in 0..180 {
        let status = operation(pic, root, operation_id);
        if predicate(&status) {
            return status;
        }
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
    }
    panic!(
        "child allocation did not reach the expected boundary: {:?}",
        operation(pic, root, operation_id)
    );
}
