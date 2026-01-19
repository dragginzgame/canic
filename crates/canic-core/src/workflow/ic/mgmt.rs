use crate::{
    InternalError,
    dto::canister::{
        CanisterSettings, CanisterStatusResponse, CanisterStatusType, EnvironmentVariable,
        LogVisibility, MemoryMetrics, QueryStats,
    },
    ops::ic::mgmt::{
        CanisterSettingsSnapshot, CanisterStatus, CanisterStatusType as MgmtCanisterStatusType,
        EnvironmentVariable as MgmtEnvironmentVariable, LogVisibility as MgmtLogVisibility,
        MemoryMetricsSnapshot, MgmtOps, QueryStatsSnapshot,
    },
    workflow::prelude::*,
};

///
/// MgmtAdapter
///

pub struct MgmtAdapter;

impl MgmtAdapter {
    #[must_use]
    pub fn canister_status_to_dto(status: CanisterStatus) -> CanisterStatusResponse {
        CanisterStatusResponse {
            status: Self::status_type_to_dto(status.status),
            settings: Self::settings_to_dto(status.settings),
            module_hash: status.module_hash,
            memory_size: status.memory_size,
            memory_metrics: Self::memory_metrics_to_dto(status.memory_metrics),
            cycles: status.cycles,
            reserved_cycles: status.reserved_cycles,
            idle_cycles_burned_per_day: status.idle_cycles_burned_per_day,
            query_stats: Self::query_stats_to_dto(status.query_stats),
        }
    }

    const fn status_type_to_dto(status: MgmtCanisterStatusType) -> CanisterStatusType {
        match status {
            MgmtCanisterStatusType::Running => CanisterStatusType::Running,
            MgmtCanisterStatusType::Stopping => CanisterStatusType::Stopping,
            MgmtCanisterStatusType::Stopped => CanisterStatusType::Stopped,
        }
    }

    fn settings_to_dto(settings: CanisterSettingsSnapshot) -> CanisterSettings {
        CanisterSettings {
            controllers: settings.controllers,
            compute_allocation: settings.compute_allocation,
            memory_allocation: settings.memory_allocation,
            freezing_threshold: settings.freezing_threshold,
            reserved_cycles_limit: settings.reserved_cycles_limit,
            log_visibility: Self::log_visibility_to_dto(settings.log_visibility),
            wasm_memory_limit: settings.wasm_memory_limit,
            wasm_memory_threshold: settings.wasm_memory_threshold,
            environment_variables: settings
                .environment_variables
                .into_iter()
                .map(Self::environment_variable_to_dto)
                .collect(),
        }
    }

    fn log_visibility_to_dto(log_visibility: MgmtLogVisibility) -> LogVisibility {
        match log_visibility {
            MgmtLogVisibility::Controllers => LogVisibility::Controllers,
            MgmtLogVisibility::Public => LogVisibility::Public,
            MgmtLogVisibility::AllowedViewers(viewers) => LogVisibility::AllowedViewers(viewers),
        }
    }

    fn environment_variable_to_dto(variable: MgmtEnvironmentVariable) -> EnvironmentVariable {
        EnvironmentVariable {
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
}

///
/// MgmtWorkflow
///

pub struct MgmtWorkflow;

impl MgmtWorkflow {
    pub async fn canister_status(pid: Principal) -> Result<CanisterStatusResponse, InternalError> {
        let status = MgmtOps::canister_status(pid).await?;

        Ok(MgmtAdapter::canister_status_to_dto(status))
    }
}
