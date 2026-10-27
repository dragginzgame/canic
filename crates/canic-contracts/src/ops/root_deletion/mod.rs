//! Lossless deletion boundary conversion; no cycle, controller or journal policy.
use crate::dto::{fleet_registry as stored, root_deletion as wire};
impl From<stored::FleetSubnetRootDeletionExecutionRequest> for wire::PrepareRequest {
    fn from(value: stored::FleetSubnetRootDeletionExecutionRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            expected_readiness_hash: value.expected_readiness_hash,
            observed_module_hash: value.observed_module_hash,
            observed_controllers: value
                .observed_controllers
                .into_iter()
                .map(Into::into)
                .collect(),
            observed_cycles_after_reclamation: value.observed_cycles_after_reclamation,
            observed_reserved_cycles: value.observed_reserved_cycles,
            observed_idle_cycles_burned_per_day: value.observed_idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds: value.observed_freezing_threshold_seconds,
        }
    }
}
impl From<wire::PrepareRequest> for stored::FleetSubnetRootDeletionExecutionRequest {
    fn from(value: wire::PrepareRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            expected_readiness_hash: value.expected_readiness_hash,
            observed_module_hash: value.observed_module_hash,
            observed_controllers: value
                .observed_controllers
                .into_iter()
                .map(Into::into)
                .collect(),
            observed_cycles_after_reclamation: value.observed_cycles_after_reclamation,
            observed_reserved_cycles: value.observed_reserved_cycles,
            observed_idle_cycles_burned_per_day: value.observed_idle_cycles_burned_per_day,
            observed_freezing_threshold_seconds: value.observed_freezing_threshold_seconds,
        }
    }
}
impl From<stored::FleetSubnetRootDeletionExecutionResponse> for wire::Execution {
    fn from(value: stored::FleetSubnetRootDeletionExecutionResponse) -> Self {
        Self {
            request: value.request.into(),
            executor: value.executor.into(),
            prepared_at_ns: value.prepared_at_ns,
            execution_hash: value.execution_hash,
        }
    }
}
impl From<wire::Execution> for stored::FleetSubnetRootDeletionExecutionResponse {
    fn from(value: wire::Execution) -> Self {
        Self {
            request: value.request.into(),
            executor: value.executor.into(),
            prepared_at_ns: value.prepared_at_ns,
            execution_hash: value.execution_hash,
        }
    }
}
impl From<stored::FleetSubnetRootDeletionCompletionRequest> for wire::CompleteRequest {
    fn from(value: stored::FleetSubnetRootDeletionCompletionRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            expected_execution_hash: value.expected_execution_hash,
            observed_absent_at_ns: value.observed_absent_at_ns,
        }
    }
}
impl From<wire::CompleteRequest> for stored::FleetSubnetRootDeletionCompletionRequest {
    fn from(value: wire::CompleteRequest) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            expected_execution_hash: value.expected_execution_hash,
            observed_absent_at_ns: value.observed_absent_at_ns,
        }
    }
}
impl From<stored::FleetSubnetRootDeletionResponse> for wire::Receipt {
    fn from(value: stored::FleetSubnetRootDeletionResponse) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            coordinator: value.coordinator.into(),
            executor: value.executor.into(),
            readiness_hash: value.readiness_hash,
            execution_hash: value.execution_hash,
            observed_module_hash: value.observed_module_hash,
            observed_controllers: value
                .observed_controllers
                .into_iter()
                .map(Into::into)
                .collect(),
            observed_cycles_after_reclamation: value.observed_cycles_after_reclamation,
            observed_absent_at_ns: value.observed_absent_at_ns,
            completed_at_ns: value.completed_at_ns,
            deletion_hash: value.deletion_hash,
        }
    }
}
impl From<wire::Receipt> for stored::FleetSubnetRootDeletionResponse {
    fn from(value: wire::Receipt) -> Self {
        Self {
            operation_id: value.operation_id,
            fleet_subnet_root: value.fleet_subnet_root.into(),
            coordinator: value.coordinator.into(),
            executor: value.executor.into(),
            readiness_hash: value.readiness_hash,
            execution_hash: value.execution_hash,
            observed_module_hash: value.observed_module_hash,
            observed_controllers: value
                .observed_controllers
                .into_iter()
                .map(Into::into)
                .collect(),
            observed_cycles_after_reclamation: value.observed_cycles_after_reclamation,
            observed_absent_at_ns: value.observed_absent_at_ns,
            completed_at_ns: value.completed_at_ns,
            deletion_hash: value.deletion_hash,
        }
    }
}
