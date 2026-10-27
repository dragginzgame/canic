//! Qualify a retained low-native-reserve child claim and its same-operation recovery.

use super::*;
use canic::dto::component_registry::{
    ComponentRegistryPartitionRequest, RootComponentChildAllocationRequest,
};
use canic_contracts::dto::component_registry::RootComponentChildAllocationResponse;

#[test]
pub(super) fn initial_child_failure_reaches_coordinator_and_recovers_same_claim() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace =
        crate::workspace::discover().expect("discover Canic test workspace through Cargo");
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
    let registry =
        crate::pic::registry_discovery::snapshot(&pic, coordinator, Principal::anonymous())
            .unwrap();
    let operation_id = [0x93; 32];
    let plan = fixture_fresh_component_plan(config.model(), &registry, operation_id);
    crate::pic::batch_control::prepare(
        &pic,
        coordinator,
        Principal::anonymous(),
        plan.request.clone(),
    )
    .unwrap();
    wait_for_parent_commit(&pic, root, operation_id);
    assert_balance_control_requires_controller(&pic, root);
    let burned: u128 = lower_root_reserve(&pic, root);
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
        canic_contracts::diagnostics::codes::DEPLOYMENT_CYCLE_RESERVE_REQUIRED
            .raw_code()
            .raw()
    );
    let child = operation(&pic, root, origin.operation_id);
    assert_eq!(child.phase, RootComponentAllocationPhase::Reserved);
    assert!(child.creation.is_none());
    assert!(child.installation.is_none());
    let claim = root_pool_status(&pic, root).entries.into_iter().find(|entry| {
        matches!(&entry.status, CanisterPoolAssetStatus::Claimed { claim } if claim.operation_id == origin.operation_id)
    }).expect("the same failed initial claim remains reserved");
    // Let the accepted parent runtime retry membership while its initial child
    // remains blocked. The host must still report the exact child failure.
    pic.advance_time(Duration::from_secs(30));
    pic.tick();
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
    assert!(recovered.last_failure.is_none());
    assert_eq!(
        recovered.creation.unwrap().canister,
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
    let workspace =
        crate::workspace::discover().expect("discover Canic test workspace through Cargo");
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
    let wire_schema = directory.join("coordinator.wire.json");
    std::fs::write(
        &wire_schema,
        crate::pic::artifacts::build_canonical_fleet_coordinator_schema(&workspace),
    )
    .expect("retain installed Coordinator schema");
    let action = EnsureAction::FleetProtocol {
        action: Box::new(CurrentFleetProtocolAction::ProvisionComponents {
            plan_hash: plan.plan_hash,
            request: plan.request.clone(),
        }),
        wire_schema: wire_schema.to_str().unwrap().into(),
        wire_schema_sha256: hex_bytes(wasm_hash(&std::fs::read(&wire_schema).unwrap())),
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
            ic_wire::native::encode_request(
                crate::pic::application_probe::AUDIT_ROOT_BALANCE.method_id,
                &0_u128,
                crate::pic::application_probe::AUDIT_ROOT_BALANCE.request_limits,
            )
            .unwrap(),
        )
        .expect_err("a non-controller cannot burn the audit Root's balance");
    assert_eq!(
        denied.reject_code,
        ic_platform::pocket_ic::RejectCode::CanisterReject
    );
}

fn wait_for_parent_commit(pic: &PocketIc, root: Principal, operation_id: [u8; 32]) {
    for _ in 0..240 {
        if let Ok(status) =
            crate::pic::canic::root_provisioning(pic, root, Principal::anonymous(), operation_id)
            && status.component_count > 0
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
    crate::pic::canic::coordinator_provisioning(
        pic,
        coordinator,
        Principal::anonymous(),
        operation_id,
    )
    .unwrap()
}

#[test]
pub(super) fn low_native_reserve_retains_child_failure_and_recovers_same_claim() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace =
        crate::workspace::discover().expect("discover Canic test workspace through Cargo");
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
    let partition = root_partition(
        &pic,
        root,
        ComponentRegistryPartitionRequest {
            component: parent.component,
        },
    )
    .unwrap();
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
        status.last_failure.is_some()
    });
    let failure = blocked.last_failure.unwrap();
    assert_eq!(
        failure.diagnostic_code,
        canic_contracts::diagnostics::codes::DEPLOYMENT_CYCLE_RESERVE_REQUIRED
            .raw_code()
            .raw()
    );
    assert_eq!(blocked.phase, RootComponentAllocationPhase::Reserved);
    assert!(blocked.creation.is_none());
    assert!(blocked.installation.is_none());
    let claim = root_pool_status(&pic, root).entries.into_iter().find(|entry| {
        matches!(&entry.status, CanisterPoolAssetStatus::Claimed { claim } if claim.operation_id == request.operation_id)
    }).expect("retained pool claim");
    pic.add_cycles(root, 5_000_000_000_000);
    let recovered = wait_for(&pic, root, request.operation_id, |status| {
        status.last_failure.is_none()
            && status.phase == RootComponentAllocationPhase::Committed
            && status
                .creation
                .as_ref()
                .and_then(|creation| creation.canister)
                .is_some_and(|child| {
                    fixture_readiness(&pic, root, child).status
                        == canic::dto::runtime::ReadinessStatus::Ready
                })
    });
    assert_eq!(
        recovered.creation.as_ref().unwrap().canister,
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
        let response = import_canister(
            pic,
            root,
            PoolCanisterRequest {
                canister_id: spare.canister_id,
            },
        );
        match response {
            Ok(PoolImportResponse::Imported { .. }) => {}
            // Background maintenance may already own this physical reset.
            // Retry only its transient contention result; authority conflicts fail.
            Err(error) => assert_eq!(
                error.code(),
                canic_contracts::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
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
    let paused = crate::pic::protected_state::set_funding(
        pic,
        root,
        Principal::anonymous(),
        canic::dto::state::SetCyclesFundingRequest { enabled: false },
    )
    .unwrap();
    assert_eq!(paused.propagation.unconfirmed_targets, 0);
    assert!(paused.reconciliation_error.is_none());
    let burned: u128 = ic_testkit::pic::update_wire(
        pic,
        root,
        Principal::anonymous(),
        crate::pic::application_probe::AUDIT_ROOT_BALANCE,
        &800_000_000_000_u128,
    )
    .expect("update Candid harness call");
    assert!(burned > 0);
    assert!(pic.cycle_balance(root) < 1_000_000_000_000);
}

fn provision(
    pic: &PocketIc,
    root: Principal,
    parent: Principal,
    request: &RootComponentChildAllocationRequest,
) {
    use canic::protocol::component_control as wire;
    let response: Result<canic::dto::role::OperationReceipt, Error> = ic_testkit::pic::update_wire(
        pic,
        root,
        parent,
        ic_testkit::pic::WireCallSpec {
            method: wire::child::METHOD,
            method_id: wire::child::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        request,
    )
    .expect("native child provisioning transport");
    let receipt = response.unwrap();
    assert_eq!(receipt.operation_id, request.operation_id);
}

fn operation(
    pic: &PocketIc,
    root: Principal,
    operation_id: [u8; 32],
) -> RootComponentChildAllocationResponse {
    crate::pic::canic::root_child_allocation(pic, root, Principal::anonymous(), operation_id)
        .expect("native child allocation status")
}

fn wait_for(
    pic: &PocketIc,
    root: Principal,
    operation_id: [u8; 32],
    predicate: impl Fn(&RootComponentChildAllocationResponse) -> bool,
) -> RootComponentChildAllocationResponse {
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

fn lower_root_reserve(pic: &PocketIc, root: Principal) -> u128 {
    ic_testkit::pic::update_wire(
        pic,
        root,
        Principal::anonymous(),
        crate::pic::application_probe::AUDIT_ROOT_BALANCE,
        &800_000_000_000_u128,
    )
    .expect("update Candid harness call")
}
