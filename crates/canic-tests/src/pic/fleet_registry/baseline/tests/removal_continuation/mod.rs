//! Installed native retirement custody, reconstruction and exact replay.

use super::*;
use canic_contracts::dto::component_registry::{
    RootComponentDrainingRequest, RootComponentSubtreeRemovalRequest,
};

#[test]
pub(super) fn component_removal_reconstructs_without_caller_replay() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace = crate::workspace::discover().unwrap();
    let config_path = initial_shard_root_canister_config_path(&workspace);
    let pic = build_pic();
    let coordinator = pic.create_canister().unwrap();
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES)
        .unwrap();
    let fixture = install_initial_shard_fixture(&pic, coordinator, &config_path, &[]);
    let pic = &pic;
    let root = fixture.root_id;
    reset_prepaid_pool_assets(pic, root);
    let (version, synchronization) = join_and_synchronize_root(pic, coordinator, &fixture);
    for _ in 0..=ic_timers::MAX_TIMER_REGISTRATIONS {
        assert_eq!(
            crate::pic::registry_discovery::synchronize(pic, root, synchronization.clone())
                .unwrap()
                .operation_id,
            synchronization.operation_id
        );
    }
    reconstruct(pic, root, fixture.response.wasm_store, None);
    pic.start_canister(fixture.response.wasm_store, Some(root))
        .unwrap();
    activate_registry_and_prepare_component_registry(
        pic,
        coordinator,
        &fixture,
        version,
        synchronization.clone(),
    );
    assert_eq!(
        crate::pic::registry_discovery::synchronize(pic, root, synchronization.clone())
            .unwrap()
            .operation_id,
        synchronization.operation_id
    );
    activate_root(pic, root);
    assert!(
        !root_runtime(pic, root)
            .timers
            .iter()
            .any(|timer| timer.owner == "canic" && timer.subsystem == "root_transition")
    );
    let before = root_pool_status(pic, root).workload;
    let allocation_id = [0xda; 32];
    let allocated = allocate_after_reconstruction(
        pic,
        root,
        fixture.response.wasm_store,
        RootComponentAllocationRequest {
            operation_id: allocation_id,
            component_spec: "indexed".parse().unwrap(),
        },
    );
    let partition = root_partition(
        pic,
        root,
        canic::dto::component_registry::ComponentRegistryPartitionRequest {
            component: allocated.component,
        },
    )
    .unwrap();
    let request = RootComponentDrainingRequest {
        operation_id: [0xdb; 32],
        component: allocated.component,
        expected_registry: partition.head,
    };
    let receipt = remove_component(pic, root, &request).unwrap();
    reconstruct(
        pic,
        root,
        fixture.response.wasm_store,
        Some("component_retirement"),
    );
    pic.start_canister(fixture.response.wasm_store, Some(root))
        .unwrap();
    let mut completed = false;
    for _ in 0..180 {
        let observed = crate::pic::canic::root_component_allocation(
            pic,
            root,
            Principal::anonymous(),
            allocation_id,
        )
        .unwrap();
        assert_eq!(observed.allocation.component, allocated.component);
        if observed.allocation.phase == RootComponentAllocationPhase::Removed {
            completed = true;
            break;
        }
        pic.advance_time(Duration::from_secs(1)).unwrap();
        pic.tick().unwrap();
    }
    assert!(
        completed,
        "reconstructed Component removal must finish autonomously"
    );
    assert_eq!(root_pool_status(pic, root).workload, before);
    assert_eq!(remove_component(pic, root, &request).unwrap(), receipt);
    assert_eq!(root_pool_status(pic, root).workload, before);
}

pub(super) fn qualify_subtree(pic: &PocketIc, root: Principal, store: Principal, child: Principal) {
    let ManagedCanisterBinding::ComponentChild(binding) = managed_binding_status(pic, root, child)
    else {
        panic!("selected child retains its Component binding");
    };
    let component = binding.component.component;
    let partition = root_partition(
        pic,
        root,
        canic::dto::component_registry::ComponentRegistryPartitionRequest { component },
    )
    .unwrap();
    let before = root_pool_status(pic, root).workload;
    let request = RootComponentSubtreeRemovalRequest {
        operation_id: [0xdc; 32],
        component,
        target_canister_id: child,
        expected_registry: partition.head,
    };
    let receipt = remove_subtree(pic, root, request.clone()).unwrap();
    reconstruct(pic, root, store, Some("component_retirement"));
    pic.start_canister(store, Some(root)).unwrap();
    wait_for_fixture_subtree_removal(pic, root, request.operation_id);
    assert_eq!(root_pool_status(pic, root).workload, before - 1);
    assert_eq!(remove_subtree(pic, root, request).unwrap(), receipt);
    assert_eq!(root_pool_status(pic, root).workload, before - 1);
}

fn allocate_after_reconstruction(
    pic: &PocketIc,
    root: Principal,
    store: Principal,
    request: RootComponentAllocationRequest,
) -> RootComponentAllocationResponse {
    let receipt =
        crate::pic::canic::provision_component(pic, root, Principal::anonymous(), request.clone())
            .unwrap();
    reconstruct(pic, root, store, Some("component_allocation"));
    exhaust_and_resume_allocation(pic, root, store, request.clone());
    for _ in 0..80 {
        let observed = crate::pic::canic::root_component_allocation(
            pic,
            root,
            Principal::anonymous(),
            request.operation_id,
        )
        .unwrap();
        if observed.complete {
            assert_eq!(
                observed.allocation.phase,
                RootComponentAllocationPhase::Committed
            );
            assert_eq!(
                crate::pic::canic::provision_component(pic, root, Principal::anonymous(), request,)
                    .unwrap(),
                receipt
            );
            return observed.allocation;
        }
        pic.advance_time(Duration::from_secs(1)).unwrap();
        pic.tick().unwrap();
    }
    panic!("reconstructed ordinary allocation must finish without caller replay");
}

fn reconstruct(pic: &PocketIc, root: Principal, store: Principal, subsystem: Option<&str>) {
    pic.stop_canister(store, Some(root)).unwrap();
    pic.stop_canister(root, None).unwrap();
    pic.upgrade_canister(
        root,
        build_initial_shard_root_wasm(),
        crate::pic::upgrade_args(),
        None,
    )
    .unwrap();
    pic.start_canister(root, None).unwrap();
    let Some(subsystem) = subsystem else {
        return;
    };
    let runtime = root_runtime(pic, root);
    let owners = runtime
        .timers
        .iter()
        .filter(|timer| {
            timer.owner == "canic" && timer.subsystem == subsystem && timer.name == "advance"
        })
        .collect::<Vec<_>>();
    assert_eq!(owners.len(), 1);
    assert!(matches!(
        owners[0].registration,
        TimerRegistrationStatus::Scheduled | TimerRegistrationStatus::Running
    ));
}

fn exhaust_and_resume_allocation(
    pic: &PocketIc,
    root: Principal,
    store: Principal,
    request: RootComponentAllocationRequest,
) {
    use canic::dto::operation_retry::{Operation, ResumeRequest, State};
    let operation = Operation::RootAllocation {
        operation_id: request.operation_id,
    };
    for _ in 0..64 {
        let status = crate::pic::protected_state::retry_status(pic, root, operation).unwrap();
        if status.state == State::BudgetExhausted {
            break;
        }
        pic.advance_time(Duration::from_secs(61)).unwrap();
        for _ in 0..8 {
            pic.tick().unwrap();
        }
    }
    let exhausted = crate::pic::protected_state::retry_status(pic, root, operation).unwrap();
    assert_eq!(exhausted.state, State::BudgetExhausted);
    assert_eq!(
        crate::pic::protected_state::prepare_snapshot(
            pic,
            root,
            Principal::anonymous(),
            canic::dto::authority_restore::AuthoritySnapshotRequest {
                operation_id: [0xdd; 32]
            },
        )
        .unwrap_err()
        .code(),
        canic_contracts::diagnostics::codes::STATE_UNAVAILABLE.raw_code()
    );
    crate::pic::canic::provision_component(pic, root, Principal::anonymous(), request).unwrap();
    assert_eq!(
        crate::pic::protected_state::retry_status(pic, root, operation).unwrap(),
        exhausted
    );
    pic.stop_canister(root, None).unwrap();
    pic.upgrade_canister(
        root,
        build_initial_shard_root_wasm(),
        crate::pic::upgrade_args(),
        None,
    )
    .unwrap();
    pic.start_canister(root, None).unwrap();
    assert_eq!(
        crate::pic::protected_state::retry_status(pic, root, operation).unwrap(),
        exhausted
    );
    pic.start_canister(store, Some(root)).unwrap();
    pic.advance_time(Duration::from_secs(61)).unwrap();
    for _ in 0..16 {
        pic.tick().unwrap();
    }
    assert_eq!(
        crate::pic::protected_state::retry_status(pic, root, operation).unwrap(),
        exhausted
    );
    let resumed = crate::pic::protected_state::resume_retry(
        pic,
        root,
        ResumeRequest {
            operation,
            expected_generation: exhausted.generation,
        },
    )
    .unwrap();
    assert_eq!(resumed.generation, exhausted.generation + 1);
}

fn remove_component(
    pic: &PocketIc,
    root: Principal,
    request: &RootComponentDrainingRequest,
) -> Result<canic::dto::role::OperationReceipt, Error> {
    use canic::protocol::component_control as wire;
    ic_testkit::pic::update_wire(
        pic,
        root,
        Principal::anonymous(),
        ic_testkit::pic::WireCallSpec {
            method: wire::remove::METHOD,
            method_id: wire::remove::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        request,
    )
    .unwrap()
}
