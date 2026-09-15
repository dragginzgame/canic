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
use canic_control_plane::dto::template::{WasmStoreGcRequest, WasmStoreGcTarget};

#[derive(CandidType)]
enum StateCommand {
    MaintainPool,
    SetCyclesFunding(SetCyclesFundingRequest),
    SetFleetStatus(SetFleetStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum StateResponse {
    SetCyclesFunding(FleetStateCommandResult<bool>),
    SetFleetStatus(FleetStateCommandResult<FleetStatus>),
}

#[derive(CandidType)]
enum StateQuery {
    FleetState,
}

#[derive(CandidType, Deserialize)]
enum StateQueryResponse {
    FleetState(FleetStateResponse),
}

#[derive(CandidType)]
enum SnapshotCommand {
    SynchronizeState(StateSnapshotInput),
}

#[derive(CandidType, Deserialize)]
enum SnapshotResponse {
    SynchronizeState(StateCascadeReport),
}

#[derive(CandidType)]
enum ManagedAdminCommand {
    OpenFleetAdmission(canic::dto::fleet_admission::FleetAdmissionOpenTargetRequest),
}

#[derive(CandidType)]
enum StoreAdminCommand {
    RunGc(WasmStoreGcRequest),
}

#[test]
pub(super) fn live_state_cascade_preserves_partial_outcomes_and_retry() {
    let _serial = crate::pic::acquire_pic_unit_test_serial_guard();
    let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
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
        command(
            &pic,
            root,
            outsider,
            StateCommand::SetCyclesFunding(SetCyclesFundingRequest { enabled: false }),
        ),
        canic_core::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code(),
    );
    for (target, caller, method) in [
        (shard, root, canic::protocol::CANIC_COMMAND),
        (store, outsider, canic::protocol::CANIC_WASM_STORE_COMMAND),
    ] {
        let response: Result<SnapshotResponse, Error> = pic.update_candid_as_or_panic(
            target,
            caller,
            method,
            (SnapshotCommand::SynchronizeState(StateSnapshotInput {
                fleet_state: Some(FleetStateInput {
                    mode: FleetMode::Readonly,
                    cycles_funding_enabled: false,
                }),
            }),),
        );
        assert_error(
            response,
            canic_core::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code(),
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
        let StateResponse::SetFleetStatus(response) = command(
            pic,
            root,
            Principal::anonymous(),
            StateCommand::SetFleetStatus(SetFleetStatusRequest { status }),
        )
        .unwrap() else {
            panic!("correlated status response")
        };
        assert_eq!(response.change.current, status);
        assert_complete(&response.propagation, expected);
        let code = match status {
            FleetStatus::Readonly => canic_core::diagnostics::codes::AUTHORITY_INVALID_STATE,
            FleetStatus::Stopped => canic_core::diagnostics::codes::AUTHORITY_INACTIVE,
            FleetStatus::Active => continue,
        }
        .raw_code();
        assert_error(
            command(
                pic,
                root,
                Principal::anonymous(),
                StateCommand::MaintainPool,
            ),
            code,
        );
        let store_response: Result<SnapshotResponse, Error> = pic.update_candid_as_or_panic(
            store,
            root,
            canic::protocol::CANIC_WASM_STORE_COMMAND,
            (StoreAdminCommand::RunGc(WasmStoreGcRequest {
                operation_id: [0x77; 32],
                target: WasmStoreGcTarget::Prepared,
            }),),
        );
        assert_error(store_response, code);
        let managed_response: Result<SnapshotResponse, Error> = pic.update_candid_as_or_panic(
            shard,
            root,
            canic::protocol::CANIC_COMMAND,
            (ManagedAdminCommand::OpenFleetAdmission(
                canic::dto::fleet_admission::FleetAdmissionOpenTargetRequest {
                    operation_id: [0x77; 32],
                    generation: 1,
                    policy_digest: [1; 32],
                    projection_digest: [2; 32],
                },
            ),),
        );
        assert_error(managed_response, code);
    }
}

fn assert_error<T>(response: Result<T, Error>, code: canic_core::diagnostics::DiagnosticCode) {
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
    let CoordinatorRegistryResponse::Registry(registry) =
        coordinator_status(pic, coordinator, CoordinatorRegistryRequest::Registry).unwrap();
    let operation_id = [0x76; 32];
    let plan = fixture_fresh_component_plan(config.model(), &registry, operation_id);
    coordinator_command(
        pic,
        coordinator,
        CoordinatorCommand::ProvisionComponents(plan.request),
    )
    .unwrap();
    for _ in 0..240 {
        let CoordinatorOperationReadResponse::Operation(
            CoordinatorOperationStatusResponse::ComponentProvisioning(status),
        ) = coordinator_status(
            pic,
            coordinator,
            CoordinatorOperationReadRequest::Operation(OperationStatusRequest { operation_id }),
        )
        .unwrap()
        else {
            panic!("correlated provisioning status")
        };
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

fn command(
    pic: &PocketIc,
    root: Principal,
    caller: Principal,
    command: StateCommand,
) -> Result<StateResponse, Error> {
    pic.update_candid_as_or_panic(
        root,
        caller,
        canic::protocol::CANIC_ROOT_COMMAND,
        (command,),
    )
}

fn funding(pic: &PocketIc, root: Principal, enabled: bool) -> FleetStateCommandResult<bool> {
    let StateResponse::SetCyclesFunding(response) = command(
        pic,
        root,
        Principal::anonymous(),
        StateCommand::SetCyclesFunding(SetCyclesFundingRequest { enabled }),
    )
    .unwrap() else {
        panic!("correlated funding response")
    };
    response
}

fn state(pic: &PocketIc, root: Principal) -> FleetStateResponse {
    let response: Result<StateQueryResponse, Error> = pic.query_candid_as_or_panic(
        root,
        Principal::anonymous(),
        canic::protocol::CANIC_ROOT_STATUS,
        (StateQuery::FleetState,),
    );
    let StateQueryResponse::FleetState(state) = response.unwrap();
    state
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
    let RootStatusResponseFragment::Runtime(runtime) =
        root_status(pic, root, RootStatusRequestFragment::Runtime).unwrap()
    else {
        panic!("runtime response")
    };
    let timer = runtime
        .timers
        .iter()
        .find(|timer| {
            timer.owner == "canic" && timer.subsystem == "cycles" && timer.name == "topup"
        })
        .expect("Root funding timer registration");
    assert_eq!(timer.next_due_at_ns.is_some(), enabled);
}
