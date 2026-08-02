//! Module: fleet_subnet_root_deletion
//!
//! Responsibility: resume and execute one externally controlled Fleet Subnet Root deletion.
//! Does not own: root/Coordinator durable authority, controller selection, or CLI presentation.
//! Boundary: only exact management status may authorize stop/delete, and only typed replica
//! absence may be attested to the surviving Coordinator.

#[cfg(test)]
mod tests;

use crate::{
    canister_protocol::{CanisterProtocolError, call_with_arg, query_with_arg},
    icp::{IcpCanisterStatusReport, IcpCli, IcpDiagnostic, LocalReplicaTarget},
};
use candid::Principal;
use canic_core::{
    cdk::utils::hash::decode_hex,
    dto::{
        error::ErrorCode,
        fleet_registry::{
            FleetSubnetRootDeletionCompletionRequest, FleetSubnetRootDeletionExecutionRequest,
            FleetSubnetRootDeletionExecutionResponse, FleetSubnetRootDeletionResponse,
            FleetSubnetRootDeletionStatusRequest,
        },
        fleet_subnet_root::{
            FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES,
            FLEET_SUBNET_ROOT_DELETION_MAXIMUM_RETAINED_CYCLES,
            FleetSubnetRootDeletionPreparationRequest, FleetSubnetRootDeletionPreparationResponse,
            FleetSubnetRootDeletionPreparationStatusRequest, FleetSubnetRootStoreDeletionResponse,
            FleetSubnetRootStoreDeletionStatusRequest,
        },
    },
    protocol,
};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const SECONDS_PER_DAY: u128 = 86_400;

/// Exact host and durable Canister authority for one root-deletion operation.
pub struct FleetSubnetRootDeletionHostRequest<'a> {
    pub icp_executable: &'a str,
    pub icp_root: &'a Path,
    pub environment: &'a str,
    pub local_replica: Option<&'a LocalReplicaTarget>,
    pub coordinator: Principal,
    pub fleet_subnet_root: Principal,
    pub operation_id: [u8; 32],
}

/// Durable protocol boundary at which host execution failed or must be retried.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FleetSubnetRootDeletionProtocolStage {
    Completion,
    ExecutionBegin,
    ExecutionStatus,
    Preparation,
    PreparationStatus,
    StoreDeletionStatus,
    TerminalStatus,
}

/// Typed host failure for one resumable physical-root deletion attempt.
#[derive(Debug, ThisError)]
pub enum FleetSubnetRootDeletionError {
    #[error("Fleet Subnet Root deletion request is invalid: {0}")]
    InvalidRequest(&'static str),

    #[error("invalid live Fleet Subnet Root {field}: {reason}")]
    InvalidStatus { field: &'static str, reason: String },

    #[error("Fleet Subnet Root was absent before the Coordinator retained execution intent")]
    RootAbsentBeforeExecution,

    #[error("Fleet Subnet Root is still stopping; retry the same operation")]
    RootStopping,

    #[error("Fleet Subnet Root remained running after stop: {0}")]
    StopDidNotSettle(String),

    #[error("Fleet Subnet Root remained present after delete: {0}")]
    DeleteDidNotSettle(String),

    #[error("failed to observe Fleet Subnet Root management status: {0}")]
    StatusObservation(String),

    #[error("failed to resolve the active root-deletion executor identity: {0}")]
    ExecutorIdentity(String),

    #[error(
        "active root-deletion executor {observed} differs from durable Coordinator executor {expected}"
    )]
    ExecutorMismatch {
        expected: Principal,
        observed: Principal,
    },

    #[error("Fleet Subnet Root deletion protocol failed during {stage:?}: {message}")]
    Protocol {
        stage: FleetSubnetRootDeletionProtocolStage,
        message: String,
    },

    #[error("system clock cannot produce a positive u64 Unix timestamp in nanoseconds")]
    SystemClock,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CanisterLifecycle {
    Running,
    Stopped,
    Stopping,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RootStatusEvidence {
    lifecycle: CanisterLifecycle,
    module_hash: [u8; 32],
    controllers: Vec<Principal>,
    cycles: u128,
    reserved_cycles: u128,
    idle_cycles_burned_per_day: u128,
    freezing_threshold_seconds: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RootStatusObservation {
    Absent,
    Present(RootStatusEvidence),
}

trait FleetSubnetRootDeletionAdapter {
    fn terminal_status(
        &mut self,
        request: FleetSubnetRootDeletionStatusRequest,
    ) -> Result<Option<FleetSubnetRootDeletionResponse>, FleetSubnetRootDeletionError>;

    fn executor_identity(&mut self) -> Result<Principal, FleetSubnetRootDeletionError>;

    fn execution_status(
        &mut self,
        request: FleetSubnetRootDeletionStatusRequest,
    ) -> Result<Option<FleetSubnetRootDeletionExecutionResponse>, FleetSubnetRootDeletionError>;

    fn preparation_status(
        &mut self,
        root: Principal,
        request: FleetSubnetRootDeletionPreparationStatusRequest,
    ) -> Result<Option<FleetSubnetRootDeletionPreparationResponse>, FleetSubnetRootDeletionError>;

    fn store_deletion_status(
        &mut self,
        root: Principal,
        request: FleetSubnetRootStoreDeletionStatusRequest,
    ) -> Result<FleetSubnetRootStoreDeletionResponse, FleetSubnetRootDeletionError>;

    fn prepare_root_deletion(
        &mut self,
        root: Principal,
        request: FleetSubnetRootDeletionPreparationRequest,
    ) -> Result<FleetSubnetRootDeletionPreparationResponse, FleetSubnetRootDeletionError>;

    fn begin_execution(
        &mut self,
        request: FleetSubnetRootDeletionExecutionRequest,
    ) -> Result<FleetSubnetRootDeletionExecutionResponse, FleetSubnetRootDeletionError>;

    fn complete_deletion(
        &mut self,
        request: FleetSubnetRootDeletionCompletionRequest,
    ) -> Result<FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionError>;

    fn observe_root(
        &mut self,
        root: Principal,
    ) -> Result<RootStatusObservation, FleetSubnetRootDeletionError>;

    fn stop_root(&mut self, root: Principal) -> Result<(), String>;

    fn delete_root(&mut self, root: Principal) -> Result<(), String>;

    fn now_nanos(&mut self) -> Result<u64, FleetSubnetRootDeletionError>;
}

struct IcpFleetSubnetRootDeletionAdapter {
    icp: IcpCli,
    coordinator: Principal,
}

/// Execute or resume one physical Fleet Subnet Root deletion from durable remote authority.
pub fn execute_fleet_subnet_root_deletion(
    request: FleetSubnetRootDeletionHostRequest<'_>,
) -> Result<FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionError> {
    validate_request(&request)?;
    let mut adapter = icp_adapter(&request);
    execute_with_adapter(
        &mut adapter,
        request.coordinator,
        request.fleet_subnet_root,
        request.operation_id,
    )
}

/// Prepare and retain the Coordinator execution intent without stopping the root.
///
/// A later `execute_fleet_subnet_root_deletion` call resumes exclusively from
/// this durable remote authority. Repeating preparation returns the same
/// execution intent without replaying a root call or issuing a management-canister effect.
pub fn prepare_fleet_subnet_root_deletion_execution(
    request: FleetSubnetRootDeletionHostRequest<'_>,
) -> Result<FleetSubnetRootDeletionExecutionResponse, FleetSubnetRootDeletionError> {
    validate_request(&request)?;
    let mut adapter = icp_adapter(&request);
    resolve_execution_intent(
        &mut adapter,
        request.coordinator,
        request.fleet_subnet_root,
        request.operation_id,
    )
}

fn icp_adapter(
    request: &FleetSubnetRootDeletionHostRequest<'_>,
) -> IcpFleetSubnetRootDeletionAdapter {
    let icp = IcpCli::new(
        request.icp_executable,
        Some(request.environment.to_string()),
    )
    .with_cwd(request.icp_root)
    .with_local_replica(request.local_replica.cloned());
    IcpFleetSubnetRootDeletionAdapter {
        icp,
        coordinator: request.coordinator,
    }
}

fn validate_request(
    request: &FleetSubnetRootDeletionHostRequest<'_>,
) -> Result<(), FleetSubnetRootDeletionError> {
    if request.icp_executable.trim().is_empty() || request.environment.trim().is_empty() {
        return Err(FleetSubnetRootDeletionError::InvalidRequest(
            "ICP executable and environment must be nonempty",
        ));
    }
    if request.operation_id == [0; 32] {
        return Err(FleetSubnetRootDeletionError::InvalidRequest(
            "operation_id must be nonzero",
        ));
    }
    let invalid_principals = [
        request.coordinator == Principal::anonymous(),
        request.fleet_subnet_root == Principal::anonymous(),
        request.coordinator == request.fleet_subnet_root,
    ]
    .into_iter()
    .any(|invalid| invalid);
    if invalid_principals {
        return Err(FleetSubnetRootDeletionError::InvalidRequest(
            "Coordinator and Fleet Subnet Root must be distinct non-anonymous principals",
        ));
    }
    Ok(())
}

fn execute_with_adapter(
    adapter: &mut impl FleetSubnetRootDeletionAdapter,
    coordinator: Principal,
    root: Principal,
    operation_id: [u8; 32],
) -> Result<FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionError> {
    let status_request = FleetSubnetRootDeletionStatusRequest {
        operation_id,
        fleet_subnet_root: root,
    };
    if let Some(terminal) = adapter.terminal_status(status_request)? {
        validate_terminal_identity(&terminal, coordinator, root, operation_id)?;
        return Ok(terminal);
    }

    let execution = resolve_execution_intent(adapter, coordinator, root, operation_id)?;
    drive_management_deletion(adapter, coordinator, root, operation_id, execution)
}

fn resolve_execution_intent(
    adapter: &mut impl FleetSubnetRootDeletionAdapter,
    coordinator: Principal,
    root: Principal,
    operation_id: [u8; 32],
) -> Result<FleetSubnetRootDeletionExecutionResponse, FleetSubnetRootDeletionError> {
    let executor_identity = adapter.executor_identity()?;
    let status_request = FleetSubnetRootDeletionStatusRequest {
        operation_id,
        fleet_subnet_root: root,
    };
    let execution = match adapter.execution_status(status_request)? {
        Some(execution) => {
            validate_execution_identity(&execution, root, operation_id)?;
            execution
        }
        None => prepare_execution(adapter, coordinator, root, operation_id)?,
    };
    if execution.executor != executor_identity {
        return Err(FleetSubnetRootDeletionError::ExecutorMismatch {
            expected: execution.executor,
            observed: executor_identity,
        });
    }
    Ok(execution)
}

fn prepare_execution(
    adapter: &mut impl FleetSubnetRootDeletionAdapter,
    coordinator: Principal,
    root: Principal,
    operation_id: [u8; 32],
) -> Result<FleetSubnetRootDeletionExecutionResponse, FleetSubnetRootDeletionError> {
    let preparation_request = FleetSubnetRootDeletionPreparationStatusRequest { operation_id };
    let preparation = match adapter.preparation_status(root, preparation_request)? {
        Some(preparation) => preparation,
        None => prepare_root(adapter, coordinator, root, operation_id)?,
    };
    validate_preparation_identity(&preparation, coordinator, root, operation_id)?;

    let status = require_present_before_execution(adapter.observe_root(root)?)?;
    if status.lifecycle != CanisterLifecycle::Running {
        return Err(invalid_status(
            "status",
            "root must remain running until execution intent is durable",
        ));
    }
    validate_status_against_preparation(&status, &preparation)?;
    let request = FleetSubnetRootDeletionExecutionRequest {
        operation_id,
        fleet_subnet_root: root,
        expected_readiness_hash: preparation.coordinator_readiness_hash,
        observed_module_hash: status.module_hash,
        observed_controllers: status.controllers,
        observed_cycles_after_reclamation: status.cycles,
        observed_reserved_cycles: status.reserved_cycles,
        observed_idle_cycles_burned_per_day: status.idle_cycles_burned_per_day,
        observed_freezing_threshold_seconds: status.freezing_threshold_seconds,
    };
    let execution = adapter.begin_execution(request.clone())?;
    if execution.request != request {
        return Err(protocol_invariant(
            FleetSubnetRootDeletionProtocolStage::ExecutionBegin,
            "Coordinator returned different execution authority",
        ));
    }
    validate_execution_identity(&execution, root, operation_id)?;
    Ok(execution)
}

fn prepare_root(
    adapter: &mut impl FleetSubnetRootDeletionAdapter,
    coordinator: Principal,
    root: Principal,
    operation_id: [u8; 32],
) -> Result<FleetSubnetRootDeletionPreparationResponse, FleetSubnetRootDeletionError> {
    let status = require_present_before_execution(adapter.observe_root(root)?)?;
    if status.lifecycle != CanisterLifecycle::Running {
        return Err(invalid_status(
            "status",
            "root must be running before deletion preparation",
        ));
    }
    if status.reserved_cycles != 0 {
        return Err(invalid_status(
            "reserved_cycles",
            "must be zero before deletion preparation",
        ));
    }
    let store_deletion = adapter.store_deletion_status(
        root,
        FleetSubnetRootStoreDeletionStatusRequest { operation_id },
    )?;
    let store_deletion_is_exact = [
        store_deletion.operation_id == operation_id,
        store_deletion.fleet_subnet_root == root,
        store_deletion.deletion_hash != [0; 32],
        store_deletion.completed_at_ns > 0,
    ]
    .into_iter()
    .all(|exact| exact);
    if !store_deletion_is_exact {
        return Err(protocol_invariant(
            FleetSubnetRootDeletionProtocolStage::StoreDeletionStatus,
            "Store deletion receipt differs from the requested root operation",
        ));
    }
    let maximum_cycles_to_retain = root_deletion_maximum_cycles(
        status.idle_cycles_burned_per_day,
        status.freezing_threshold_seconds,
    )?;
    let request = FleetSubnetRootDeletionPreparationRequest {
        operation_id,
        expected_store_deletion_hash: store_deletion.deletion_hash,
        maximum_cycles_to_retain,
        observed_reserved_cycles: status.reserved_cycles,
        observed_idle_cycles_burned_per_day: status.idle_cycles_burned_per_day,
        observed_freezing_threshold_seconds: status.freezing_threshold_seconds,
    };
    let preparation = adapter.prepare_root_deletion(root, request)?;
    validate_preparation_identity(&preparation, coordinator, root, operation_id)?;
    Ok(preparation)
}

fn drive_management_deletion(
    adapter: &mut impl FleetSubnetRootDeletionAdapter,
    coordinator: Principal,
    root: Principal,
    operation_id: [u8; 32],
    execution: FleetSubnetRootDeletionExecutionResponse,
) -> Result<FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionError> {
    match adapter.observe_root(root)? {
        RootStatusObservation::Absent => {
            complete_from_absence(adapter, coordinator, root, operation_id, &execution)
        }
        RootStatusObservation::Present(status) => {
            validate_status_against_execution(&status, &execution)?;
            match status.lifecycle {
                CanisterLifecycle::Running => {
                    stop_then_reconcile(adapter, coordinator, root, operation_id, &execution)
                }
                CanisterLifecycle::Stopping => Err(FleetSubnetRootDeletionError::RootStopping),
                CanisterLifecycle::Stopped => {
                    delete_then_reconcile(adapter, coordinator, root, operation_id, &execution)
                }
            }
        }
    }
}

fn stop_then_reconcile(
    adapter: &mut impl FleetSubnetRootDeletionAdapter,
    coordinator: Principal,
    root: Principal,
    operation_id: [u8; 32],
    execution: &FleetSubnetRootDeletionExecutionResponse,
) -> Result<FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionError> {
    let stop_error = adapter.stop_root(root).err();
    match adapter.observe_root(root)? {
        RootStatusObservation::Absent => {
            complete_from_absence(adapter, coordinator, root, operation_id, execution)
        }
        RootStatusObservation::Present(status) => {
            validate_status_against_execution(&status, execution)?;
            match status.lifecycle {
                CanisterLifecycle::Stopped => {
                    delete_then_reconcile(adapter, coordinator, root, operation_id, execution)
                }
                CanisterLifecycle::Stopping => Err(FleetSubnetRootDeletionError::RootStopping),
                CanisterLifecycle::Running => Err(FleetSubnetRootDeletionError::StopDidNotSettle(
                    stop_error.unwrap_or_else(|| {
                        "stop command succeeded but live status remained Running".to_string()
                    }),
                )),
            }
        }
    }
}

fn delete_then_reconcile(
    adapter: &mut impl FleetSubnetRootDeletionAdapter,
    coordinator: Principal,
    root: Principal,
    operation_id: [u8; 32],
    execution: &FleetSubnetRootDeletionExecutionResponse,
) -> Result<FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionError> {
    let delete_error = adapter.delete_root(root).err();
    match adapter.observe_root(root)? {
        RootStatusObservation::Absent => {
            complete_from_absence(adapter, coordinator, root, operation_id, execution)
        }
        RootStatusObservation::Present(status) => {
            validate_status_against_execution(&status, execution)?;
            Err(FleetSubnetRootDeletionError::DeleteDidNotSettle(
                delete_error.unwrap_or_else(|| {
                    format!(
                        "delete command succeeded but live status remained {:?}",
                        status.lifecycle
                    )
                }),
            ))
        }
    }
}

fn complete_from_absence(
    adapter: &mut impl FleetSubnetRootDeletionAdapter,
    coordinator: Principal,
    root: Principal,
    operation_id: [u8; 32],
    execution: &FleetSubnetRootDeletionExecutionResponse,
) -> Result<FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionError> {
    let request = FleetSubnetRootDeletionCompletionRequest {
        operation_id,
        fleet_subnet_root: root,
        expected_execution_hash: execution.execution_hash,
        observed_absent_at_ns: adapter.now_nanos()?,
    };
    let terminal = adapter.complete_deletion(request)?;
    validate_terminal_against_execution(&terminal, coordinator, execution)?;
    Ok(terminal)
}

fn require_present_before_execution(
    observation: RootStatusObservation,
) -> Result<RootStatusEvidence, FleetSubnetRootDeletionError> {
    match observation {
        RootStatusObservation::Absent => {
            Err(FleetSubnetRootDeletionError::RootAbsentBeforeExecution)
        }
        RootStatusObservation::Present(status) => Ok(status),
    }
}

fn validate_preparation_identity(
    preparation: &FleetSubnetRootDeletionPreparationResponse,
    coordinator: Principal,
    root: Principal,
    operation_id: [u8; 32],
) -> Result<(), FleetSubnetRootDeletionError> {
    let expected_maximum = root_deletion_maximum_cycles(
        preparation.observed_idle_cycles_burned_per_day,
        preparation.observed_freezing_threshold_seconds,
    )?;
    let valid = [
        preparation.operation_id == operation_id,
        preparation.fleet_subnet_root == root,
        preparation.coordinator == coordinator,
        preparation.final_inventory_hash != [0; 32],
        preparation.store_deletion_hash != [0; 32],
        preparation.observed_cycles_before_reclamation > 0,
        preparation.maximum_cycles_to_retain == expected_maximum,
        preparation.observed_reserved_cycles == 0,
        preparation.observed_cycles_after_reclamation
            <= preparation.observed_cycles_before_reclamation,
        preparation.observed_cycles_after_reclamation <= preparation.maximum_cycles_to_retain,
        preparation.coordinator_intent_hash != [0; 32],
        preparation.coordinator_readiness_hash != [0; 32],
        preparation.completed_at_ns > 0,
    ]
    .into_iter()
    .all(|item| item);
    if !valid {
        return Err(protocol_invariant(
            FleetSubnetRootDeletionProtocolStage::PreparationStatus,
            "root returned incomplete or conflicting deletion preparation authority",
        ));
    }
    Ok(())
}

fn validate_execution_identity(
    execution: &FleetSubnetRootDeletionExecutionResponse,
    root: Principal,
    operation_id: [u8; 32],
) -> Result<(), FleetSubnetRootDeletionError> {
    let request = &execution.request;
    let valid = [
        request.operation_id == operation_id,
        request.fleet_subnet_root == root,
        request.expected_readiness_hash != [0; 32],
        request.observed_module_hash != [0; 32],
        canonical_controller_set(&request.observed_controllers),
        request.observed_controllers.contains(&execution.executor),
        request.observed_reserved_cycles == 0,
        execution.prepared_at_ns > 0,
        execution.execution_hash != [0; 32],
    ]
    .into_iter()
    .all(|item| item);
    if !valid {
        return Err(protocol_invariant(
            FleetSubnetRootDeletionProtocolStage::ExecutionStatus,
            "Coordinator returned incomplete or conflicting execution authority",
        ));
    }
    Ok(())
}

fn validate_terminal_identity(
    terminal: &FleetSubnetRootDeletionResponse,
    coordinator: Principal,
    root: Principal,
    operation_id: [u8; 32],
) -> Result<(), FleetSubnetRootDeletionError> {
    let valid = [
        terminal.operation_id == operation_id,
        terminal.fleet_subnet_root == root,
        terminal.coordinator == coordinator,
        terminal.executor != Principal::anonymous(),
        terminal.readiness_hash != [0; 32],
        terminal.execution_hash != [0; 32],
        terminal.observed_module_hash != [0; 32],
        canonical_controller_set(&terminal.observed_controllers),
        terminal.observed_controllers.contains(&terminal.executor),
        terminal.observed_absent_at_ns > 0,
        terminal.completed_at_ns >= terminal.observed_absent_at_ns,
        terminal.deletion_hash != [0; 32],
    ]
    .into_iter()
    .all(|item| item);
    if !valid {
        return Err(protocol_invariant(
            FleetSubnetRootDeletionProtocolStage::TerminalStatus,
            "Coordinator returned incomplete or conflicting terminal deletion authority",
        ));
    }
    Ok(())
}

fn validate_terminal_against_execution(
    terminal: &FleetSubnetRootDeletionResponse,
    coordinator: Principal,
    execution: &FleetSubnetRootDeletionExecutionResponse,
) -> Result<(), FleetSubnetRootDeletionError> {
    validate_terminal_identity(
        terminal,
        coordinator,
        execution.request.fleet_subnet_root,
        execution.request.operation_id,
    )?;
    let exact = [
        terminal.executor == execution.executor,
        terminal.readiness_hash == execution.request.expected_readiness_hash,
        terminal.execution_hash == execution.execution_hash,
        terminal.observed_module_hash == execution.request.observed_module_hash,
        terminal.observed_controllers == execution.request.observed_controllers,
        terminal.observed_cycles_after_reclamation
            == execution.request.observed_cycles_after_reclamation,
    ]
    .into_iter()
    .all(|item| item);
    if !exact {
        return Err(protocol_invariant(
            FleetSubnetRootDeletionProtocolStage::Completion,
            "terminal receipt differs from the durable execution intent",
        ));
    }
    Ok(())
}

fn validate_status_against_preparation(
    status: &RootStatusEvidence,
    preparation: &FleetSubnetRootDeletionPreparationResponse,
) -> Result<(), FleetSubnetRootDeletionError> {
    let exact = [
        status.reserved_cycles == preparation.observed_reserved_cycles,
        status.idle_cycles_burned_per_day == preparation.observed_idle_cycles_burned_per_day,
        status.freezing_threshold_seconds == preparation.observed_freezing_threshold_seconds,
        status.cycles <= preparation.observed_cycles_before_reclamation,
        status.cycles <= preparation.maximum_cycles_to_retain,
    ]
    .into_iter()
    .all(|item| item);
    if !exact {
        return Err(invalid_status(
            "cycle authority",
            "live execution observation differs from durable root preparation",
        ));
    }
    Ok(())
}

fn validate_status_against_execution(
    status: &RootStatusEvidence,
    execution: &FleetSubnetRootDeletionExecutionResponse,
) -> Result<(), FleetSubnetRootDeletionError> {
    let expected = &execution.request;
    let exact = [
        status.module_hash == expected.observed_module_hash,
        status.controllers == expected.observed_controllers,
        status.cycles <= expected.observed_cycles_after_reclamation,
        status.reserved_cycles == expected.observed_reserved_cycles,
        status.idle_cycles_burned_per_day == expected.observed_idle_cycles_burned_per_day,
        status.freezing_threshold_seconds == expected.observed_freezing_threshold_seconds,
    ]
    .into_iter()
    .all(|item| item);
    if !exact {
        return Err(invalid_status(
            "frozen execution authority",
            "live module, controllers, cycles, or freezing evidence drifted before deletion",
        ));
    }
    Ok(())
}

fn root_deletion_maximum_cycles(
    idle_cycles_burned_per_day: u128,
    freezing_threshold_seconds: u128,
) -> Result<u128, FleetSubnetRootDeletionError> {
    let freezing_reserve = idle_cycles_burned_per_day
        .checked_mul(freezing_threshold_seconds)
        .ok_or_else(|| invalid_status("cycle authority", "freezing reserve overflows u128"))?
        .div_ceil(SECONDS_PER_DAY);
    let maximum = freezing_reserve
        .checked_add(FLEET_SUBNET_ROOT_DELETION_EXECUTION_RESERVE_CYCLES)
        .ok_or_else(|| invalid_status("cycle authority", "deletion reserve overflows u128"))?;
    if maximum > FLEET_SUBNET_ROOT_DELETION_MAXIMUM_RETAINED_CYCLES {
        return Err(invalid_status(
            "cycle authority",
            "deletion reserve exceeds the supported 1T ceiling",
        ));
    }
    Ok(maximum)
}

fn canonical_controller_set(controllers: &[Principal]) -> bool {
    !controllers.is_empty()
        && controllers
            .iter()
            .all(|controller| *controller != Principal::anonymous())
        && controllers.windows(2).all(|pair| pair[0] < pair[1])
}

impl FleetSubnetRootDeletionAdapter for IcpFleetSubnetRootDeletionAdapter {
    fn terminal_status(
        &mut self,
        request: FleetSubnetRootDeletionStatusRequest,
    ) -> Result<Option<FleetSubnetRootDeletionResponse>, FleetSubnetRootDeletionError> {
        query_optional(
            &self.icp,
            self.coordinator,
            protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_STATUS,
            &request,
            FleetSubnetRootDeletionProtocolStage::TerminalStatus,
        )
    }

    fn executor_identity(&mut self) -> Result<Principal, FleetSubnetRootDeletionError> {
        let text = self
            .icp
            .identity_principal_text()
            .map_err(|error| FleetSubnetRootDeletionError::ExecutorIdentity(error.to_string()))?;
        Principal::from_text(&text).map_err(|_| {
            FleetSubnetRootDeletionError::ExecutorIdentity(format!(
                "ICP CLI returned invalid Principal {text:?}"
            ))
        })
    }

    fn execution_status(
        &mut self,
        request: FleetSubnetRootDeletionStatusRequest,
    ) -> Result<Option<FleetSubnetRootDeletionExecutionResponse>, FleetSubnetRootDeletionError>
    {
        query_optional(
            &self.icp,
            self.coordinator,
            protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_EXECUTION_STATUS,
            &request,
            FleetSubnetRootDeletionProtocolStage::ExecutionStatus,
        )
    }

    fn preparation_status(
        &mut self,
        root: Principal,
        request: FleetSubnetRootDeletionPreparationStatusRequest,
    ) -> Result<Option<FleetSubnetRootDeletionPreparationResponse>, FleetSubnetRootDeletionError>
    {
        query_optional(
            &self.icp,
            root,
            protocol::CANIC_FLEET_SUBNET_ROOT_DELETION_PREPARATION_STATUS,
            &request,
            FleetSubnetRootDeletionProtocolStage::PreparationStatus,
        )
    }

    fn store_deletion_status(
        &mut self,
        root: Principal,
        request: FleetSubnetRootStoreDeletionStatusRequest,
    ) -> Result<FleetSubnetRootStoreDeletionResponse, FleetSubnetRootDeletionError> {
        query_protocol(
            &self.icp,
            root,
            protocol::CANIC_FLEET_SUBNET_ROOT_STORE_DELETION_STATUS,
            &request,
            FleetSubnetRootDeletionProtocolStage::StoreDeletionStatus,
        )
    }

    fn prepare_root_deletion(
        &mut self,
        root: Principal,
        request: FleetSubnetRootDeletionPreparationRequest,
    ) -> Result<FleetSubnetRootDeletionPreparationResponse, FleetSubnetRootDeletionError> {
        call_protocol(
            &self.icp,
            root,
            protocol::CANIC_FLEET_SUBNET_ROOT_DELETION_PREPARE,
            &request,
            FleetSubnetRootDeletionProtocolStage::Preparation,
        )
    }

    fn begin_execution(
        &mut self,
        request: FleetSubnetRootDeletionExecutionRequest,
    ) -> Result<FleetSubnetRootDeletionExecutionResponse, FleetSubnetRootDeletionError> {
        call_protocol(
            &self.icp,
            self.coordinator,
            protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_EXECUTION_BEGIN,
            &request,
            FleetSubnetRootDeletionProtocolStage::ExecutionBegin,
        )
    }

    fn complete_deletion(
        &mut self,
        request: FleetSubnetRootDeletionCompletionRequest,
    ) -> Result<FleetSubnetRootDeletionResponse, FleetSubnetRootDeletionError> {
        call_protocol(
            &self.icp,
            self.coordinator,
            protocol::CANIC_FLEET_REGISTRY_ROOT_DELETION_COMPLETE,
            &request,
            FleetSubnetRootDeletionProtocolStage::Completion,
        )
    }

    fn observe_root(
        &mut self,
        root: Principal,
    ) -> Result<RootStatusObservation, FleetSubnetRootDeletionError> {
        match self.icp.canister_status_report(&root.to_text()) {
            Ok(report) => parse_status_report(root, report).map(RootStatusObservation::Present),
            Err(error)
                if matches!(
                    error.diagnostic(),
                    Some(IcpDiagnostic::CanisterNotFound { canister })
                        if canister == root.to_text()
                ) =>
            {
                Ok(RootStatusObservation::Absent)
            }
            Err(error) => Err(FleetSubnetRootDeletionError::StatusObservation(
                error.to_string(),
            )),
        }
    }

    fn stop_root(&mut self, root: Principal) -> Result<(), String> {
        self.icp
            .stop_canister(&root.to_text())
            .map_err(|error| error.to_string())
    }

    fn delete_root(&mut self, root: Principal) -> Result<(), String> {
        self.icp
            .delete_canister_without_cycle_recovery(&root.to_text())
            .map_err(|error| error.to_string())
    }

    fn now_nanos(&mut self) -> Result<u64, FleetSubnetRootDeletionError> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| FleetSubnetRootDeletionError::SystemClock)?
            .as_nanos();
        u64::try_from(nanos)
            .ok()
            .filter(|nanos| *nanos > 0)
            .ok_or(FleetSubnetRootDeletionError::SystemClock)
    }
}

fn query_optional<I, O>(
    icp: &IcpCli,
    canister: Principal,
    method: &'static str,
    input: &I,
    stage: FleetSubnetRootDeletionProtocolStage,
) -> Result<Option<O>, FleetSubnetRootDeletionError>
where
    I: candid::CandidType,
    O: candid::CandidType + serde::de::DeserializeOwned,
{
    match query_with_arg(icp, canister, method, input) {
        Ok(response) => Ok(Some(response)),
        Err(error) if error.is_rejected_with(ErrorCode::Unavailable) => Ok(None),
        Err(error) => Err(protocol_error(stage, error)),
    }
}

fn call_protocol<I, O>(
    icp: &IcpCli,
    canister: Principal,
    method: &'static str,
    input: &I,
    stage: FleetSubnetRootDeletionProtocolStage,
) -> Result<O, FleetSubnetRootDeletionError>
where
    I: candid::CandidType,
    O: candid::CandidType + serde::de::DeserializeOwned,
{
    call_with_arg(icp, canister, method, input).map_err(|error| protocol_error(stage, error))
}

fn query_protocol<I, O>(
    icp: &IcpCli,
    canister: Principal,
    method: &'static str,
    input: &I,
    stage: FleetSubnetRootDeletionProtocolStage,
) -> Result<O, FleetSubnetRootDeletionError>
where
    I: candid::CandidType,
    O: candid::CandidType + serde::de::DeserializeOwned,
{
    query_with_arg(icp, canister, method, input).map_err(|error| protocol_error(stage, error))
}

fn parse_status_report(
    root: Principal,
    report: IcpCanisterStatusReport,
) -> Result<RootStatusEvidence, FleetSubnetRootDeletionError> {
    let observed_root = Principal::from_text(&report.id)
        .map_err(|_| invalid_status("id", "status returned an invalid Principal"))?;
    if observed_root != root {
        return Err(invalid_status(
            "id",
            "status returned a different Canister Principal",
        ));
    }
    let lifecycle = match report.status.as_str() {
        "Running" => CanisterLifecycle::Running,
        "Stopped" => CanisterLifecycle::Stopped,
        "Stopping" => CanisterLifecycle::Stopping,
        value => {
            return Err(invalid_status(
                "status",
                format!("unsupported value {value:?}"),
            ));
        }
    };
    let module_hash = parse_module_hash(report.module_hash.as_deref())?;
    let settings = report
        .settings
        .ok_or_else(|| invalid_status("settings", "controller-only settings are missing"))?;
    let mut controllers = settings
        .controllers
        .iter()
        .map(|controller| {
            Principal::from_text(controller)
                .map_err(|_| invalid_status("controllers", "contains an invalid Principal"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    controllers.sort();
    controllers.dedup();
    if !canonical_controller_set(&controllers) {
        return Err(invalid_status(
            "controllers",
            "must contain a nonempty non-anonymous controller set",
        ));
    }
    Ok(RootStatusEvidence {
        lifecycle,
        module_hash,
        controllers,
        cycles: parse_nat("cycles", report.cycles.as_deref())?,
        reserved_cycles: parse_nat("reserved_cycles", report.reserved_cycles.as_deref())?,
        idle_cycles_burned_per_day: parse_nat(
            "idle_cycles_burned_per_day",
            report.idle_cycles_burned_per_day.as_deref(),
        )?,
        freezing_threshold_seconds: parse_nat(
            "freezing_threshold",
            settings.freezing_threshold.as_deref(),
        )?,
    })
}

fn parse_module_hash(value: Option<&str>) -> Result<[u8; 32], FleetSubnetRootDeletionError> {
    let value =
        value.ok_or_else(|| invalid_status("module_hash", "installed Wasm hash is missing"))?;
    let value = value.strip_prefix("0x").unwrap_or(value);
    let bytes = decode_hex(value)
        .map_err(|error| invalid_status("module_hash", format!("invalid hex: {error}")))?;
    bytes
        .try_into()
        .map_err(|_| invalid_status("module_hash", "must contain exactly 32 bytes"))
}

fn parse_nat(
    field: &'static str,
    value: Option<&str>,
) -> Result<u128, FleetSubnetRootDeletionError> {
    let value = value.ok_or_else(|| invalid_status(field, "value is missing"))?;
    if !valid_nat_text(value) {
        return Err(invalid_status(
            field,
            format!("invalid natural number {value:?}"),
        ));
    }
    value
        .replace('_', "")
        .parse()
        .map_err(|_| invalid_status(field, format!("natural number {value:?} exceeds u128")))
}

fn valid_nat_text(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('_')
        && !value.ends_with('_')
        && value
            .chars()
            .all(|character| character.is_ascii_digit() || character == '_')
}

fn invalid_status(field: &'static str, reason: impl Into<String>) -> FleetSubnetRootDeletionError {
    FleetSubnetRootDeletionError::InvalidStatus {
        field,
        reason: reason.into(),
    }
}

fn protocol_invariant(
    stage: FleetSubnetRootDeletionProtocolStage,
    message: impl Into<String>,
) -> FleetSubnetRootDeletionError {
    FleetSubnetRootDeletionError::Protocol {
        stage,
        message: message.into(),
    }
}

fn protocol_error(
    stage: FleetSubnetRootDeletionProtocolStage,
    error: CanisterProtocolError,
) -> FleetSubnetRootDeletionError {
    FleetSubnetRootDeletionError::Protocol {
        stage,
        message: error.to_string(),
    }
}
