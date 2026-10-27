//! Module: pic::fleet_registry::baseline::tests::state_cascade
//!
//! Responsibility: qualify live role-correct state propagation and partial-failure retry.
//! Does not own: production transport, authority or timer behavior.
//! Boundary: real Root, Store, Hubs and descendants execute their maintained commands.

use super::*;
use canic::dto::{
    cascade::{StateCascadeReport, StateCascadeTargetResult, StateSnapshotInput},
    state::{
        FleetMode, FleetStateCommandResult, FleetStateInput, FleetStateResponse, FleetStatus,
        SetCyclesFundingRequest, SetFleetStatusRequest,
    },
};
use canic_contracts::dto::template::{WasmStoreGcRequest, WasmStoreGcTarget};

#[test]
pub(super) fn live_state_cascade_preserves_partial_outcomes_and_retry() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace =
        crate::workspace::discover().expect("discover Canic test workspace through Cargo");
    let config_path = initial_shard_root_canister_config_path(&workspace);
    let config = AppConfigSnapshot::load(&config_path).unwrap();
    let pic = build_pic();
    let coordinator = pic.create_canister();
    pic.add_cycles(coordinator, COORDINATOR_INSTALL_CYCLES);
    let fixture = install_initial_shard_fixture(&pic, coordinator, &config_path, &[]);
    activate_components(&pic, coordinator, &fixture, &config);
    let root = fixture.root_id;
    let store = fixture.response.wasm_store;
    let workloads = root_pool_status(&pic, root)
        .entries
        .into_iter()
        .filter(|entry| matches!(entry.status, CanisterPoolAssetStatus::Workload { .. }))
        .map(|entry| entry.canister_id)
        .collect::<Vec<_>>();
    let shard = workloads.iter().copied().find(|&canister| {
        matches!(managed_binding_status(&pic, root, canister), ManagedCanisterBinding::ComponentChild(binding) if binding.role.as_str() == "user_shard")
    }).expect("real managed Shard");
    let mut expected = workloads;
    expected.push(store);
    expected.sort();

    // Controller authority remains required at Root; Root cannot impersonate a
    // Shard's immediate parent through its managed snapshot command.
    let outsider = Principal::from_slice(&[90; 29]);
    assert_error(
        crate::pic::protected_state::set_funding(
            &pic,
            root,
            outsider,
            SetCyclesFundingRequest { enabled: false },
        ),
        canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code(),
    );
    for (target, caller) in [(shard, root), (store, outsider)] {
        let response: Result<canic::dto::fleet_state_control::CascadeReport, Error> =
            ic_testkit::pic::update_wire(
                &pic,
                target,
                caller,
                ic_testkit::pic::WireCallSpec {
                    method: canic::protocol::cascade::state::METHOD,
                    method_id: canic::protocol::cascade::state::METHOD_ID,
                    request_limits: canic::protocol::cascade::REQUEST_LIMITS,
                    response_limits: canic::protocol::cascade::RESPONSE_LIMITS,
                },
                &StateSnapshotInput {
                    fleet_state: Some(FleetStateInput {
                        mode: FleetMode::Readonly,
                        cycles_funding_enabled: false,
                    }),
                },
            )
            .expect("native state cascade transport");
        assert_error(
            response,
            canic_contracts::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code(),
        );
    }

    let paused = funding(&pic, root, false);
    assert!(!paused.change.current);
    assert!(paused.reconciliation_error.is_none());
    assert_complete(&paused.propagation, &expected);
    assert!(!state(&pic, root).cycles_funding_enabled);
    assert_root_timer(&pic, root, false);
    let replay = funding(&pic, root, false);
    assert!(!replay.change.changed);
    assert_eq!(replay.propagation, paused.propagation);

    // Stop a descendant, not a direct Root target: its Hub and all other branches
    // must still report applied state, while the exact Shard remains unconfirmed.
    pic.stop_canister(shard, Some(root)).unwrap();
    let partial = funding(&pic, root, true);
    assert!(partial.change.changed);
    assert!(partial.change.current);
    assert!(partial.reconciliation_error.is_none());
    assert_eq!(partial.propagation.unconfirmed_targets, 1);
    assert_eq!(
        partial.propagation.successful_targets,
        (expected.len() - 1) as u64
    );
    assert_eq!(partial.propagation.omitted_targets, 0);
    let failed = partial
        .propagation
        .targets
        .iter()
        .filter(|entry| matches!(entry.result, StateCascadeTargetResult::Unconfirmed(_)))
        .collect::<Vec<_>>();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].canister_id, shard);
    assert!(state(&pic, root).cycles_funding_enabled);
    assert_root_timer(&pic, root, true);
    pic.start_canister(shard, Some(root)).unwrap();
    let recovered = funding(&pic, root, true);
    assert!(!recovered.change.changed);
    assert_complete(&recovered.propagation, &expected);
    assert_eq!(funding(&pic, root, true).propagation, recovered.propagation);

    assert_mode_recovery(&pic, root, store, shard, &expected);
    assert_eq!(state(&pic, root).mode, FleetMode::Enabled);
}

fn assert_mode_recovery(
    pic: &PocketIc,
    root: Principal,
    store: Principal,
    shard: Principal,
    expected: &[Principal],
) {
    for status in [
        FleetStatus::Readonly,
        FleetStatus::Stopped,
        FleetStatus::Active,
    ] {
        let response = crate::pic::protected_state::set_status(
            pic,
            root,
            Principal::anonymous(),
            SetFleetStatusRequest { status },
        )
        .unwrap();
        assert_eq!(response.change.current, status);
        assert_complete(&response.propagation, expected);
        let code = match status {
            FleetStatus::Readonly => canic_contracts::diagnostics::codes::AUTHORITY_INVALID_STATE,
            FleetStatus::Stopped => canic_contracts::diagnostics::codes::AUTHORITY_INACTIVE,
            FleetStatus::Active => continue,
        }
        .raw_code();
        assert_error(crate::pic::canic::maintain_pool(pic, root), code);
        let store_response: Result<canic::dto::role::OperationReceipt, Error> =
            ic_testkit::pic::update_wire(
                pic,
                store,
                root,
                ic_testkit::pic::WireCallSpec {
                    method: canic::protocol::store_gc::METHOD,
                    method_id: canic::protocol::store_gc::METHOD_ID,
                    request_limits: canic::protocol::store_gc::REQUEST_LIMITS,
                    response_limits: canic::protocol::store_gc::RESPONSE_LIMITS,
                },
                &WasmStoreGcRequest {
                    operation_id: [0x77; 32],
                    target: WasmStoreGcTarget::Prepared,
                },
            )
            .expect("native Store GC transport");
        assert_error(store_response, code);
        let managed_response: Result<canic::dto::admission_control::TargetReceipt, Error> =
            ic_testkit::pic::update_wire(
                pic,
                shard,
                root,
                ic_testkit::pic::WireCallSpec {
                    method: canic::protocol::admission_control::target_open::METHOD,
                    method_id: canic::protocol::admission_control::target_open::METHOD_ID,
                    request_limits: canic::protocol::admission_control::REQUEST_LIMITS,
                    response_limits: canic::protocol::admission_control::RESPONSE_LIMITS,
                },
                &canic::dto::fleet_admission::FleetAdmissionOpenTargetRequest {
                    operation_id: [0x77; 32],
                    generation: 1,
                    policy_digest: [1; 32],
                    projection_digest: [2; 32],
                },
            )
            .expect("native managed transition");
        assert_error(managed_response, code);
    }
}

fn assert_error<T>(response: Result<T, Error>, code: canic_contracts::diagnostics::DiagnosticCode) {
    assert_eq!(
        response.err().expect("typed command rejection").code(),
        code
    );
}

pub(super) fn activate_components(
    pic: &PocketIc,
    coordinator: Principal,
    fixture: &BootstrappedRootFixture,
    config: &AppConfigSnapshot,
) {
    let (version, sync) = join_and_synchronize_root(pic, coordinator, fixture);
    activate_registry_and_prepare_component_registry(pic, coordinator, fixture, version, sync);
    let registry =
        crate::pic::registry_discovery::snapshot(pic, coordinator, Principal::anonymous()).unwrap();
    let operation_id = [0x76; 32];
    let plan = fixture_fresh_component_plan(config.model(), &registry, operation_id);
    crate::pic::batch_control::prepare(pic, coordinator, Principal::anonymous(), plan.request)
        .unwrap();
    for _ in 0..240 {
        let status = crate::pic::canic::coordinator_provisioning(
            pic,
            coordinator,
            Principal::anonymous(),
            operation_id,
        )
        .unwrap();
        if status.components_provisioned_at_ns.is_some()
            && status.runtimes_activated_at_ns.is_some()
            && status.runtime_activated_root_count == status.root_batch_count
        {
            return;
        }
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
    }
    panic!("fixture activation must complete before exercising live state changes");
}

fn funding(pic: &PocketIc, root: Principal, enabled: bool) -> FleetStateCommandResult<bool> {
    crate::pic::protected_state::set_funding(
        pic,
        root,
        Principal::anonymous(),
        SetCyclesFundingRequest { enabled },
    )
    .unwrap()
}

fn state(pic: &PocketIc, root: Principal) -> FleetStateResponse {
    let response = crate::pic::protected_state::fleet_state(pic, root, Principal::anonymous());
    response.expect("Root Fleet state")
}

fn assert_complete(report: &StateCascadeReport, expected: &[Principal]) {
    assert_eq!(report.successful_targets, expected.len() as u64);
    assert_eq!(report.reconciliation_failures, 0);
    assert_eq!(report.unconfirmed_targets, 0);
    assert_eq!(report.omitted_targets, 0);
    let mut observed = report
        .targets
        .iter()
        .map(|entry| {
            assert_eq!(entry.result, StateCascadeTargetResult::Applied);
            entry.canister_id
        })
        .collect::<Vec<_>>();
    observed.sort();
    assert_eq!(observed, expected);
}

fn assert_root_timer(pic: &PocketIc, root: Principal, enabled: bool) {
    let runtime = root_runtime(pic, root);
    let timer = runtime
        .timers
        .iter()
        .find(|timer| {
            timer.owner == "canic" && timer.subsystem == "cycles" && timer.name == "topup"
        })
        .expect("Root funding timer registration");
    assert_eq!(timer.next_due_at_ns.is_some(), enabled);
}
