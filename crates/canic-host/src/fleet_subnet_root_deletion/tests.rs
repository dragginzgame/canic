use super::*;
use crate::icp::IcpCanisterStatusSettings;
use std::collections::VecDeque;

const OPERATION_ID: [u8; 32] = [7; 32];
const RETAINED_CYCLES: u128 = FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES + 1;

struct ScriptedAdapter {
    terminal: Option<FleetSubnetRootDeletionResponse>,
    execution: Option<FleetSubnetRootDeletionExecutionResponse>,
    preparation: Option<FleetSubnetRootDeletionPreparationResponse>,
    observations: VecDeque<RootStatusObservation>,
    stop_result: Result<(), String>,
    delete_result: Result<(), String>,
    executor_identity: Principal,
    events: Vec<&'static str>,
}

impl ScriptedAdapter {
    fn with_observations(observations: impl IntoIterator<Item = RootStatusObservation>) -> Self {
        Self {
            terminal: None,
            execution: None,
            preparation: None,
            observations: observations.into_iter().collect(),
            stop_result: Ok(()),
            delete_result: Ok(()),
            executor_identity: executor(),
            events: Vec::new(),
        }
    }
}

impl FleetSubnetRootDeletionAdapter for ScriptedAdapter {
    fn terminal_status(
        &mut self,
        request: FleetSubnetRootDeletionStatusRequest,
    ) -> Result<Option<FleetSubnetRootDeletionResponse>, FleetSubnetRootDeletionError> {
        assert_status_request(request);
        self.events.push("terminal_status");
        Ok(self.terminal.clone())
    }

    fn executor_identity(&mut self) -> Result<Principal, FleetSubnetRootDeletionError> {
        self.events.push("executor_identity");
        Ok(self.executor_identity)
    }

    fn execution_status(
        &mut self,
        request: FleetSubnetRootDeletionStatusRequest,
    ) -> Result<Option<FleetSubnetRootDeletionExecutionResponse>, FleetSubnetRootDeletionError>
    {
        assert_status_request(request);
        self.events.push("execution_status");
        Ok(self.execution.clone())
    }

    fn preparation_status(
        &mut self,
        observed_root: Principal,
        request: FleetSubnetRootDeletionPreparationStatusRequest,
    ) -> Result<Option<FleetSubnetRootDeletionPreparationResponse>, FleetSubnetRootDeletionError>
    {
        assert_eq!(observed_root, root());
        assert_eq!(request.operation_id, OPERATION_ID);
        self.events.push("preparation_status");
        Ok(self.preparation.clone())
    }

    fn store_deletion_status(
        &mut self,
        observed_root: Principal,
        request: FleetSubnetRootStoreDeletionStatusRequest,
    ) -> Result<FleetSubnetRootStoreDeletionResponse, FleetSubnetRootDeletionError> {
        assert_eq!(observed_root, root());
        assert_eq!(request.operation_id, OPERATION_ID);
        self.events.push("store_deletion_status");
        Ok(store_deletion())
    }

    fn prepare_root_deletion(
        &mut self,
        observed_root: Principal,
        request: FleetSubnetRootDeletionPreparationRequest,
    ) -> Result<FleetSubnetRootDeletionPreparationResponse, FleetSubnetRootDeletionError> {
        assert_eq!(observed_root, root());
        assert_eq!(request.operation_id, OPERATION_ID);
        assert_eq!(request.expected_store_deletion_hash, [6; 32]);
        assert_eq!(request.maximum_cycles_to_retain, RETAINED_CYCLES);
        assert_eq!(request.observed_reserved_cycles, 0);
        self.events.push("prepare");
        let response = preparation();
        self.preparation = Some(response.clone());
        Ok(response)
    }

    fn begin_execution(
        &mut self,
        request: FleetSubnetRootDeletionExecutionRequest,
    ) -> Result<FleetSubnetRootDeletionExecutionResponse, FleetSubnetRootDeletionError> {
        assert_eq!(request.operation_id, OPERATION_ID);
        assert_eq!(request.fleet_subnet_root, root());
        self.events.push("begin_execution");
        let response = execution_from(request);
        self.execution = Some(response.clone());
        Ok(response)
    }

    fn complete_deletion(
        &mut self,
        request: FleetSubnetRootDeletionCompletionRequest,
    ) -> Result<FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionError> {
        assert_eq!(request.operation_id, OPERATION_ID);
        assert_eq!(request.fleet_subnet_root, root());
        self.events.push("complete");
        let response = terminal_from(
            self.execution
                .as_ref()
                .expect("execution intent is retained"),
            request.observed_absent_at_ns,
        );
        self.terminal = Some(response.clone());
        Ok(response)
    }

    fn observe_root(
        &mut self,
        observed_root: Principal,
    ) -> Result<RootStatusObservation, FleetSubnetRootDeletionError> {
        assert_eq!(observed_root, root());
        self.events.push("observe");
        Ok(self
            .observations
            .pop_front()
            .expect("scripted status observation"))
    }

    fn stop_root(&mut self, observed_root: Principal) -> Result<(), String> {
        assert_eq!(observed_root, root());
        self.events.push("stop");
        self.stop_result.clone()
    }

    fn delete_root(&mut self, observed_root: Principal) -> Result<(), String> {
        assert_eq!(observed_root, root());
        self.events.push("delete");
        self.delete_result.clone()
    }

    fn now_nanos(&mut self) -> Result<u64, FleetSubnetRootDeletionError> {
        self.events.push("now");
        Ok(1_000)
    }
}

#[test]
fn full_execution_prepares_stops_deletes_and_attests_exact_absence() {
    let mut adapter = ScriptedAdapter::with_observations([
        present(CanisterLifecycle::Running, 500_000_000_000),
        present(CanisterLifecycle::Running, 100_000_000_000),
        present(CanisterLifecycle::Running, 100_000_000_000),
        present(CanisterLifecycle::Stopped, 100_000_000_000),
        RootStatusObservation::Absent,
    ]);

    let terminal = execute(&mut adapter).expect("execute physical root deletion");

    assert_eq!(terminal.fleet_subnet_root, root());
    assert_eq!(terminal.observed_absent_at_ns, 1_000);
    assert_eq!(
        adapter.events,
        [
            "terminal_status",
            "executor_identity",
            "execution_status",
            "preparation_status",
            "observe",
            "store_deletion_status",
            "prepare",
            "observe",
            "begin_execution",
            "observe",
            "stop",
            "observe",
            "delete",
            "observe",
            "now",
            "complete",
        ]
    );
}

#[test]
fn durable_execution_resumes_from_stopped_without_replaying_preparation_or_stop() {
    let mut adapter = ScriptedAdapter::with_observations([
        present(CanisterLifecycle::Stopped, 100_000_000_000),
        RootStatusObservation::Absent,
    ]);
    adapter.execution = Some(execution());

    execute(&mut adapter).expect("resume stopped root deletion");

    assert_eq!(
        adapter.events,
        [
            "terminal_status",
            "executor_identity",
            "execution_status",
            "observe",
            "delete",
            "observe",
            "now",
            "complete",
        ]
    );
}

#[test]
fn lost_stop_or_delete_response_is_adopted_only_from_later_exact_status() {
    let mut adapter = ScriptedAdapter::with_observations([
        present(CanisterLifecycle::Running, 100_000_000_000),
        present(CanisterLifecycle::Stopped, 100_000_000_000),
        RootStatusObservation::Absent,
    ]);
    adapter.execution = Some(execution());
    adapter.stop_result = Err("lost stop response".to_string());
    adapter.delete_result = Err("lost delete response".to_string());

    execute(&mut adapter).expect("adopt independently observed stop and absence");

    assert!(adapter.events.contains(&"stop"));
    assert!(adapter.events.contains(&"delete"));
    assert!(adapter.events.contains(&"complete"));
}

#[test]
fn absent_root_without_durable_execution_intent_fails_closed() {
    let mut adapter = ScriptedAdapter::with_observations([RootStatusObservation::Absent]);

    let error = execute(&mut adapter).expect_err("absence cannot create execution authority");

    assert!(matches!(
        error,
        FleetSubnetRootDeletionError::RootAbsentBeforeExecution
    ));
    assert!(!adapter.events.contains(&"complete"));
}

#[test]
fn reserved_cycles_fail_before_store_receipt_or_cycle_transfer() {
    let mut evidence = status(CanisterLifecycle::Running, 500_000_000_000);
    evidence.reserved_cycles = 1;
    let mut adapter =
        ScriptedAdapter::with_observations([RootStatusObservation::Present(evidence)]);

    let error = execute(&mut adapter).expect_err("reserved cycles must fail preflight");

    assert!(matches!(
        error,
        FleetSubnetRootDeletionError::InvalidStatus {
            field: "reserved_cycles",
            ..
        }
    ));
    assert!(!adapter.events.contains(&"store_deletion_status"));
    assert!(!adapter.events.contains(&"prepare"));
}

#[test]
fn stopping_root_requires_retry_without_deletion_or_attestation() {
    let mut adapter =
        ScriptedAdapter::with_observations([present(CanisterLifecycle::Stopping, 100_000_000_000)]);
    adapter.execution = Some(execution());

    let error = execute(&mut adapter).expect_err("Stopping is not typed absence");

    assert!(matches!(error, FleetSubnetRootDeletionError::RootStopping));
    assert!(!adapter.events.contains(&"delete"));
    assert!(!adapter.events.contains(&"complete"));
}

#[test]
fn authority_drift_fails_before_any_management_mutation() {
    let mut drifted = status(CanisterLifecycle::Running, 100_000_000_000);
    drifted.module_hash = [99; 32];
    let mut adapter = ScriptedAdapter::with_observations([RootStatusObservation::Present(drifted)]);
    adapter.execution = Some(execution());

    let error = execute(&mut adapter).expect_err("module drift must fail closed");

    assert!(matches!(
        error,
        FleetSubnetRootDeletionError::InvalidStatus {
            field: "frozen execution authority",
            ..
        }
    ));
    assert!(!adapter.events.contains(&"stop"));
}

#[test]
fn a_different_controller_cannot_take_over_the_frozen_executor() {
    let mut adapter = ScriptedAdapter::with_observations([]);
    adapter.execution = Some(execution());
    adapter.executor_identity = controller();

    let error = execute(&mut adapter).expect_err("different controller cannot take over");

    assert!(matches!(
        error,
        FleetSubnetRootDeletionError::ExecutorMismatch { .. }
    ));
    assert!(!adapter.events.contains(&"observe"));
    assert!(!adapter.events.contains(&"stop"));
    assert!(!adapter.events.contains(&"delete"));
}

#[test]
fn terminal_status_short_circuits_every_root_operation() {
    let execution = execution();
    let expected = terminal_from(&execution, 1_000);
    let mut adapter = ScriptedAdapter::with_observations([]);
    adapter.terminal = Some(expected.clone());

    let observed = execute(&mut adapter).expect("return durable terminal receipt");

    assert_eq!(observed, expected);
    assert_eq!(adapter.events, ["terminal_status"]);
}

#[test]
fn parses_private_icp_status_into_canonical_execution_evidence() {
    let report = IcpCanisterStatusReport {
        id: root().to_text(),
        name: Some("root".to_string()),
        status: "Running".to_string(),
        settings: Some(IcpCanisterStatusSettings {
            controllers: vec![controller().to_text(), executor().to_text()],
            compute_allocation: Some("0".to_string()),
            memory_allocation: None,
            freezing_threshold: Some("1".to_string()),
            reserved_cycles_limit: None,
            wasm_memory_limit: None,
            wasm_memory_threshold: None,
            log_memory_limit: None,
        }),
        module_hash: Some(format!("0x{}", "09".repeat(32))),
        memory_size: None,
        cycles: Some("100_000_000_000".to_string()),
        reserved_cycles: Some("0".to_string()),
        idle_cycles_burned_per_day: Some("86_400".to_string()),
    };

    let parsed = parse_status_report(root(), report).expect("parse exact private status");

    assert_eq!(parsed, status(CanisterLifecycle::Running, 100_000_000_000));
}

fn execute(
    adapter: &mut ScriptedAdapter,
) -> Result<FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionError> {
    execute_with_adapter(adapter, coordinator(), root(), OPERATION_ID)
}

fn assert_status_request(request: FleetSubnetRootDeletionStatusRequest) {
    assert_eq!(request.operation_id, OPERATION_ID);
    assert_eq!(request.fleet_subnet_root, root());
}

fn present(lifecycle: CanisterLifecycle, cycles: u128) -> RootStatusObservation {
    RootStatusObservation::Present(status(lifecycle, cycles))
}

fn status(lifecycle: CanisterLifecycle, cycles: u128) -> RootStatusEvidence {
    let mut controllers = vec![controller(), executor()];
    controllers.sort();
    RootStatusEvidence {
        lifecycle,
        module_hash: [9; 32],
        controllers,
        cycles,
        reserved_cycles: 0,
        idle_cycles_burned_per_day: 86_400,
        freezing_threshold_seconds: 1,
    }
}

fn preparation() -> FleetSubnetRootDeletionPreparationResponse {
    FleetSubnetRootDeletionPreparationResponse {
        operation_id: OPERATION_ID,
        fleet_subnet_root: root(),
        coordinator: coordinator(),
        final_inventory_hash: [5; 32],
        store_deletion_hash: [6; 32],
        observed_cycles_before_reclamation: 500_000_000_000,
        maximum_cycles_to_retain: RETAINED_CYCLES,
        observed_reserved_cycles: 0,
        observed_idle_cycles_burned_per_day: 86_400,
        observed_freezing_threshold_seconds: 1,
        observed_cycles_after_reclamation: 100_000_000_000,
        cycles_reclaimed_at_ns: 20,
        coordinator_intent_hash: [10; 32],
        coordinator_readiness_hash: [11; 32],
        prepared_at_ns: 10,
        completed_at_ns: 30,
    }
}

fn store_deletion() -> FleetSubnetRootStoreDeletionResponse {
    FleetSubnetRootStoreDeletionResponse {
        operation_id: OPERATION_ID,
        fleet_subnet_root: root(),
        wasm_store: Principal::from_slice(&[4; 29]),
        binding_finalization_hash: [4; 32],
        observed_module_hash: [5; 32],
        observed_controllers: vec![root()],
        observed_cycles_before_reclamation: 10,
        maximum_cycles_to_retain: 9,
        observed_cycles_after_reclamation: 8,
        cycles_reclaimed_at_ns: 4,
        prepared_at_ns: 5,
        observed_absent_at_ns: 6,
        completed_at_ns: 7,
        deletion_hash: [6; 32],
    }
}

fn execution() -> FleetSubnetRootDeletionExecutionResponse {
    let status = status(CanisterLifecycle::Running, 100_000_000_000);
    execution_from(FleetSubnetRootDeletionExecutionRequest {
        operation_id: OPERATION_ID,
        fleet_subnet_root: root(),
        expected_readiness_hash: [11; 32],
        observed_module_hash: status.module_hash,
        observed_controllers: status.controllers,
        observed_cycles_after_reclamation: status.cycles,
        observed_reserved_cycles: status.reserved_cycles,
        observed_idle_cycles_burned_per_day: status.idle_cycles_burned_per_day,
        observed_freezing_threshold_seconds: status.freezing_threshold_seconds,
    })
}

fn execution_from(
    request: FleetSubnetRootDeletionExecutionRequest,
) -> FleetSubnetRootDeletionExecutionResponse {
    FleetSubnetRootDeletionExecutionResponse {
        request,
        executor: executor(),
        prepared_at_ns: 40,
        execution_hash: [12; 32],
    }
}

fn terminal_from(
    execution: &FleetSubnetRootDeletionExecutionResponse,
    observed_absent_at_ns: u64,
) -> FleetSubnetRootDeletionResponse {
    FleetSubnetRootDeletionResponse {
        operation_id: execution.request.operation_id,
        fleet_subnet_root: execution.request.fleet_subnet_root,
        coordinator: coordinator(),
        executor: execution.executor,
        readiness_hash: execution.request.expected_readiness_hash,
        execution_hash: execution.execution_hash,
        observed_module_hash: execution.request.observed_module_hash,
        observed_controllers: execution.request.observed_controllers.clone(),
        observed_cycles_after_reclamation: execution.request.observed_cycles_after_reclamation,
        observed_absent_at_ns,
        completed_at_ns: observed_absent_at_ns + 1,
        deletion_hash: [13; 32],
    }
}

fn coordinator() -> Principal {
    Principal::from_slice(&[1; 29])
}

fn root() -> Principal {
    Principal::from_slice(&[2; 29])
}

fn executor() -> Principal {
    Principal::from_slice(&[3; 29])
}

fn controller() -> Principal {
    Principal::from_slice(&[4; 29])
}
