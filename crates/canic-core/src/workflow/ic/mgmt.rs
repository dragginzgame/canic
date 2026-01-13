use crate::{
    InternalError,
    dto::canister::{
        CanisterSettingsView, CanisterStatusTypeView, CanisterStatusView, EnvironmentVariableView,
        LogVisibilityView, MemoryMetricsView, QueryStatsView,
    },
    ops::ic::mgmt::{
        CanisterSettingsSnapshot, CanisterStatus, CanisterStatusType, EnvironmentVariable,
        LogVisibility, MemoryMetricsSnapshot, MgmtOps, QueryStatsSnapshot,
    },
    workflow::prelude::*,
};

///
/// MgmtAdapter
///

pub struct MgmtAdapter;

impl MgmtAdapter {
    #[must_use]
    pub fn canister_status_to_view(status: CanisterStatus) -> CanisterStatusView {
        CanisterStatusView {
            status: Self::status_type_to_view(status.status),
            settings: Self::settings_to_view(status.settings),
            module_hash: status.module_hash,
            memory_size: status.memory_size,
            memory_metrics: Self::memory_metrics_to_view(status.memory_metrics),
            cycles: status.cycles,
            reserved_cycles: status.reserved_cycles,
            idle_cycles_burned_per_day: status.idle_cycles_burned_per_day,
            query_stats: Self::query_stats_to_view(status.query_stats),
        }
    }

    const fn status_type_to_view(status: CanisterStatusType) -> CanisterStatusTypeView {
        match status {
            CanisterStatusType::Running => CanisterStatusTypeView::Running,
            CanisterStatusType::Stopping => CanisterStatusTypeView::Stopping,
            CanisterStatusType::Stopped => CanisterStatusTypeView::Stopped,
        }
    }

    fn settings_to_view(settings: CanisterSettingsSnapshot) -> CanisterSettingsView {
        CanisterSettingsView {
            controllers: settings.controllers,
            compute_allocation: settings.compute_allocation,
            memory_allocation: settings.memory_allocation,
            freezing_threshold: settings.freezing_threshold,
            reserved_cycles_limit: settings.reserved_cycles_limit,
            log_visibility: Self::log_visibility_to_view(settings.log_visibility),
            wasm_memory_limit: settings.wasm_memory_limit,
            wasm_memory_threshold: settings.wasm_memory_threshold,
            environment_variables: settings
                .environment_variables
                .into_iter()
                .map(Self::environment_variable_to_view)
                .collect(),
        }
    }

    fn log_visibility_to_view(log_visibility: LogVisibility) -> LogVisibilityView {
        match log_visibility {
            LogVisibility::Controllers => LogVisibilityView::Controllers,
            LogVisibility::Public => LogVisibilityView::Public,
            LogVisibility::AllowedViewers(viewers) => LogVisibilityView::AllowedViewers(viewers),
        }
    }

    fn environment_variable_to_view(variable: EnvironmentVariable) -> EnvironmentVariableView {
        EnvironmentVariableView {
            name: variable.name,
            value: variable.value,
        }
    }

    fn memory_metrics_to_view(metrics: MemoryMetricsSnapshot) -> MemoryMetricsView {
        MemoryMetricsView {
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

    fn query_stats_to_view(stats: QueryStatsSnapshot) -> QueryStatsView {
        QueryStatsView {
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
    pub async fn canister_status_view(pid: Principal) -> Result<CanisterStatusView, InternalError> {
        let status = MgmtOps::canister_status(pid).await?;

        Ok(MgmtAdapter::canister_status_to_view(status))
    }
}
