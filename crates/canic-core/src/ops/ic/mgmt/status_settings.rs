use super::*;

impl MgmtOps {
    #[must_use]
    pub fn canister_status_to_dto(status: CanisterStatus) -> CanisterStatusResponse {
        CanisterStatusResponse {
            status: status_type_to_dto(status.status),
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

const fn status_type_to_dto(status: CanisterStatusType) -> CanisterStatusTypeDto {
    match status {
        CanisterStatusType::Running => CanisterStatusTypeDto::Running,
        CanisterStatusType::Stopping => CanisterStatusTypeDto::Stopping,
        CanisterStatusType::Stopped => CanisterStatusTypeDto::Stopped,
    }
}

fn settings_to_dto(settings: CanisterSettingsSnapshot) -> CanisterSettingsDto {
    CanisterSettingsDto {
        controllers: settings.controllers,
        compute_allocation: settings.compute_allocation,
        memory_allocation: settings.memory_allocation,
        freezing_threshold: settings.freezing_threshold,
        reserved_cycles_limit: settings.reserved_cycles_limit,
        log_visibility: log_visibility_to_dto(settings.log_visibility),
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

fn log_visibility_to_dto(log_visibility: LogVisibility) -> LogVisibilityDto {
    match log_visibility {
        LogVisibility::Controllers => LogVisibilityDto::Controllers,
        LogVisibility::Public => LogVisibilityDto::Public,
        LogVisibility::AllowedViewers(viewers) => LogVisibilityDto::AllowedViewers(viewers),
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
