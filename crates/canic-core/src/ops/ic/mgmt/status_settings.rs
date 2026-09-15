//! Module: ops::ic::mgmt::status_settings
//!
//! Responsibility: expose canister status and settings management calls.
//! Does not own: settings policy, endpoint DTO schema, or lifecycle workflow.
//! Boundary: `MgmtOps` extension for status/settings calls and DTO projection.

use super::*;
use crate::dto::canister::{
    CanisterHistoryResponse, CanisterInspectionOutcome, CanisterInspectionReserveResponse,
};

impl MgmtOps {
    /// Sample native/liquid balances and the exact call reserve without an IC call.
    pub fn canister_inspection_reserve(
        canister_pid: Principal,
    ) -> Result<CanisterInspectionReserveResponse, InternalError> {
        let required_liquid_cycles =
            MgmtInfra::canister_status_call_cost(canister_pid).map_err(OpsError::from)?;
        Ok(CanisterInspectionReserveResponse {
            caller: crate::ops::ic::IcOps::canister_self(),
            canister_id: canister_pid,
            native_cycles: ic_cdk::api::canister_cycle_balance(),
            available_liquid_cycles: ic_cdk::api::canister_liquid_cycle_balance(),
            required_liquid_cycles,
        })
    }

    /// Preserve replicated history and its exact requested target for host recovery.
    pub async fn canister_history(
        canister_pid: Principal,
    ) -> Result<CanisterHistoryResponse, InternalError> {
        let response = management_call(
            ManagementCallMetricOperation::CanisterInfo,
            MgmtInfra::canister_history(canister_pid),
        )
        .await?;
        Ok(CanisterHistoryResponse {
            canister_id: canister_pid,
            history_candid: response.into_bytes(),
        })
    }

    /// Preserve exact SDK reserve admission evidence for a controller-owned inspection.
    pub async fn canister_inspection(
        canister_pid: Principal,
    ) -> Result<CanisterInspectionOutcome, InternalError> {
        let native_cycles = crate::ops::ic::IcOps::canister_cycle_balance().to_u128();
        match management_call_infra(
            ManagementCallMetricOperation::CanisterStatus,
            MgmtInfra::canister_status(canister_pid),
        )
        .await
        {
            Ok(status) => {
                SystemMetrics::increment(SystemMetricKind::CanisterStatus);
                Ok(CanisterInspectionOutcome::Status(Box::new(
                    Self::canister_status_to_dto(canister_status_from_infra(status)),
                )))
            }
            Err(error) => inspection_failure(
                canister_pid,
                crate::ops::ic::IcOps::canister_self(),
                native_cycles,
                error,
            ),
        }
    }

    /// Observe the independent monotonic management-history count for one Canister.
    pub async fn canister_history_total_changes(
        canister_pid: Principal,
    ) -> Result<u64, InternalError> {
        management_call(
            ManagementCallMetricOperation::CanisterInfo,
            MgmtInfra::canister_history_total_changes(canister_pid),
        )
        .await
    }

    #[must_use]
    pub fn canister_status_to_dto(status: CanisterStatus) -> CanisterStatusResponse {
        CanisterStatusResponse {
            status: status.status,
            settings: settings_to_dto(status.settings),
            module_hash: status.module_hash,
            memory_size: status.memory_size,
            memory_metrics: memory_metrics_to_dto(status.memory_metrics),
            cycles: status.cycles,
            reserved_cycles: status.reserved_cycles,
            idle_cycles_burned_per_day: status.idle_cycles_burned_per_day,
            query_stats: query_stats_to_dto(status.query_stats),
        }
    }

    /// Internal ops entrypoint used by workflow and other ops helpers.
    pub async fn canister_status(canister_pid: Principal) -> Result<CanisterStatus, InternalError> {
        let status = management_call(
            ManagementCallMetricOperation::CanisterStatus,
            MgmtInfra::canister_status(canister_pid),
        )
        .await?;

        SystemMetrics::increment(SystemMetricKind::CanisterStatus);

        Ok(canister_status_from_infra(status))
    }

    /// Observe one Canister without collapsing typed absence into an infra failure.
    pub async fn observe_canister_status(
        canister_pid: Principal,
    ) -> Result<CanisterStatusObservation, InternalError> {
        match management_call_infra(
            ManagementCallMetricOperation::CanisterStatus,
            MgmtInfra::canister_status(canister_pid),
        )
        .await
        {
            Ok(status) => {
                SystemMetrics::increment(SystemMetricKind::CanisterStatus);
                Ok(CanisterStatusObservation::Present(Box::new(
                    canister_status_from_infra(status),
                )))
            }
            Err(error) if error.is_canister_not_found() => {
                SystemMetrics::increment(SystemMetricKind::CanisterStatus);
                Ok(CanisterStatusObservation::Absent)
            }
            Err(error) => Err(OpsError::from(error).into()),
        }
    }

    /// Updates canister settings via the management canister and records metrics.
    pub async fn update_settings(args: &UpdateSettingsArgs) -> Result<(), InternalError> {
        let infra_args = update_settings_to_infra(args);
        management_call(
            ManagementCallMetricOperation::UpdateSettings,
            MgmtInfra::update_settings(&infra_args),
        )
        .await?;

        SystemMetrics::increment(SystemMetricKind::UpdateSettings);

        Ok(())
    }
}

fn settings_to_dto(settings: CanisterSettingsSnapshot) -> CanisterSettingsDto {
    CanisterSettingsDto {
        controllers: settings.controllers,
        compute_allocation: settings.compute_allocation,
        memory_allocation: settings.memory_allocation,
        freezing_threshold: settings.freezing_threshold,
        reserved_cycles_limit: settings.reserved_cycles_limit,
        log_visibility: settings.log_visibility,
        log_memory_limit: settings.log_memory_limit,
        wasm_memory_limit: settings.wasm_memory_limit,
        wasm_memory_threshold: settings.wasm_memory_threshold,
        environment_variables: settings
            .environment_variables
            .into_iter()
            .map(environment_variable_to_dto)
            .collect(),
    }
}

fn environment_variable_to_dto(variable: EnvironmentVariable) -> EnvironmentVariableDto {
    EnvironmentVariableDto {
        name: variable.name,
        value: variable.value,
    }
}

fn memory_metrics_to_dto(metrics: MemoryMetricsSnapshot) -> MemoryMetrics {
    MemoryMetrics {
        wasm_memory_size: metrics.wasm_memory_size,
        stable_memory_size: metrics.stable_memory_size,
        global_memory_size: metrics.global_memory_size,
        wasm_binary_size: metrics.wasm_binary_size,
        custom_sections_size: metrics.custom_sections_size,
        canister_history_size: metrics.canister_history_size,
        wasm_chunk_store_size: metrics.wasm_chunk_store_size,
        snapshots_size: metrics.snapshots_size,
    }
}

fn query_stats_to_dto(stats: QueryStatsSnapshot) -> QueryStats {
    QueryStats {
        num_calls_total: stats.num_calls_total,
        num_instructions_total: stats.num_instructions_total,
        request_payload_bytes_total: stats.request_payload_bytes_total,
        response_payload_bytes_total: stats.response_payload_bytes_total,
    }
}

fn inspection_failure(
    canister_id: Principal,
    caller: Principal,
    native_cycles: u128,
    error: IcInfraError,
) -> Result<CanisterInspectionOutcome, InternalError> {
    match error {
        IcInfraError::CallFailed(ic_cdk::call::CallFailed::InsufficientLiquidCycleBalance(
            error,
        )) => Ok(CanisterInspectionOutcome::ReserveRequired(
            CanisterInspectionReserveResponse {
                caller,
                canister_id,
                native_cycles,
                available_liquid_cycles: error.available,
                required_liquid_cycles: error.required,
            },
        )),
        error => Err(OpsError::from(error).into()),
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::codes;
    use ic_cdk::call::{CallFailed, CallPerformFailed, InsufficientLiquidCycleBalance};

    #[test]
    fn protected_inspection_retains_exact_reserve_numbers_and_target() {
        let caller = Principal::from_slice(&[1]);
        let target = Principal::from_slice(&[2]);
        let response = inspection_failure(
            target,
            caller,
            1000,
            CallFailed::InsufficientLiquidCycleBalance(InsufficientLiquidCycleBalance {
                available: 50,
                required: 100,
            })
            .into(),
        )
        .unwrap();
        let bytes = candid::encode_one(response).unwrap();
        let decoded: CanisterInspectionOutcome = candid::decode_one(&bytes).unwrap();
        let CanisterInspectionOutcome::ReserveRequired(evidence) = decoded else {
            panic!("expected protected reserve evidence");
        };
        assert_eq!(
            evidence,
            CanisterInspectionReserveResponse {
                caller,
                canister_id: target,
                native_cycles: 1000,
                available_liquid_cycles: 50,
                required_liquid_cycles: 100,
            }
        );
        let error = inspection_failure(
            target,
            caller,
            1000,
            CallFailed::CallPerformFailed(CallPerformFailed).into(),
        )
        .err()
        .unwrap();
        assert_eq!(error.public_code(), Some(codes::PLATFORM_UNAVAILABLE));
    }
}
