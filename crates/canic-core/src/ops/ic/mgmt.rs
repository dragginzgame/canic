//! ops::ic::mgmt
//!
//! Ops-level wrappers over IC management canister calls.
//! Adds metrics, logging, and normalizes errors into `Error`.

use crate::{
    Error,
    cdk::{
        mgmt::{
            CanisterInstallMode, CanisterStatusResult, CanisterStatusType,
            DefiniteCanisterSettings, EnvironmentVariable, LogVisibility, MemoryMetrics,
            QueryStats, UpdateSettingsArgs,
        },
        types::Cycles,
    },
    dto::canister::{
        CanisterSettingsView, CanisterStatusTypeView, CanisterStatusView, EnvironmentVariableView,
        LogVisibilityView, MemoryMetricsView, QueryStatsView,
    },
    infra,
    ops::{prelude::*, runtime::metrics::system::record_system_metric},
    storage::metrics::system::SystemMetricKind,
};
use candid::{Principal, utils::ArgumentEncoder};

//
// ──────────────────────────────── CREATE CANISTER ────────────────────────────
//

/// Create a canister with explicit controllers and an initial cycle balance.
pub async fn create_canister(
    controllers: Vec<Principal>,
    cycles: Cycles,
) -> Result<Principal, Error> {
    let pid = infra::ic::mgmt::create_canister(controllers, cycles).await?;

    record_system_metric(SystemMetricKind::CreateCanister);

    Ok(pid)
}

//
// ────────────────────────────── CANISTER STATUS ──────────────────────────────
//

/// Internal ops entrypoint used by workflow and other ops helpers.
pub async fn canister_status(canister_pid: Principal) -> Result<CanisterStatusResult, Error> {
    let status = infra::ic::mgmt::canister_status(canister_pid).await?;

    record_system_metric(SystemMetricKind::CanisterStatus);

    Ok(status)
}

pub async fn canister_status_view(canister_pid: Principal) -> Result<CanisterStatusView, Error> {
    let status = canister_status(canister_pid).await?;
    Ok(canister_status_to_view(status))
}

fn canister_status_to_view(status: CanisterStatusResult) -> CanisterStatusView {
    CanisterStatusView {
        status: status_type_to_view(status.status),
        settings: settings_to_view(status.settings),
        module_hash: status.module_hash,
        memory_size: status.memory_size,
        memory_metrics: memory_metrics_to_view(status.memory_metrics),
        cycles: status.cycles,
        reserved_cycles: status.reserved_cycles,
        idle_cycles_burned_per_day: status.idle_cycles_burned_per_day,
        query_stats: query_stats_to_view(status.query_stats),
    }
}

fn status_type_to_view(status: CanisterStatusType) -> CanisterStatusTypeView {
    match status {
        CanisterStatusType::Running => CanisterStatusTypeView::Running,
        CanisterStatusType::Stopping => CanisterStatusTypeView::Stopping,
        CanisterStatusType::Stopped => CanisterStatusTypeView::Stopped,
    }
}

fn settings_to_view(settings: DefiniteCanisterSettings) -> CanisterSettingsView {
    CanisterSettingsView {
        controllers: settings.controllers,
        compute_allocation: settings.compute_allocation,
        memory_allocation: settings.memory_allocation,
        freezing_threshold: settings.freezing_threshold,
        reserved_cycles_limit: settings.reserved_cycles_limit,
        log_visibility: log_visibility_to_view(settings.log_visibility),
        wasm_memory_limit: settings.wasm_memory_limit,
        wasm_memory_threshold: settings.wasm_memory_threshold,
        environment_variables: settings
            .environment_variables
            .into_iter()
            .map(environment_variable_to_view)
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

fn memory_metrics_to_view(metrics: MemoryMetrics) -> MemoryMetricsView {
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

fn query_stats_to_view(stats: QueryStats) -> QueryStatsView {
    QueryStatsView {
        num_calls_total: stats.num_calls_total,
        num_instructions_total: stats.num_instructions_total,
        request_payload_bytes_total: stats.request_payload_bytes_total,
        response_payload_bytes_total: stats.response_payload_bytes_total,
    }
}

//
// ──────────────────────────────── CYCLES API ─────────────────────────────────
//

/// Returns the local canister's cycle balance (cheap).
#[must_use]
pub fn canister_cycle_balance() -> Cycles {
    infra::ic::mgmt::canister_cycle_balance()
}

/// Deposits cycles into a canister and records metrics.
pub async fn deposit_cycles(canister_pid: Principal, cycles: u128) -> Result<(), Error> {
    infra::ic::mgmt::deposit_cycles(canister_pid, cycles).await?;

    record_system_metric(SystemMetricKind::DepositCycles);

    Ok(())
}

/// Gets a canister's cycle balance (expensive: calls mgmt canister).
pub async fn get_cycles(canister_pid: Principal) -> Result<Cycles, Error> {
    infra::ic::mgmt::get_cycles(canister_pid)
        .await
        .map_err(Error::from)
}

//
// ──────────────────────────────── RANDOMNESS ────────────────────────────────
//

/// Query the management canister for raw randomness and record metrics.
pub async fn raw_rand() -> Result<[u8; 32], Error> {
    let seed = infra::ic::mgmt::raw_rand().await?;

    record_system_metric(SystemMetricKind::RawRand);

    Ok(seed)
}

//
// ────────────────────────────── INSTALL / UNINSTALL ──────────────────────────
//

/// Installs or upgrades a canister with the given wasm + args and records metrics.
pub async fn install_code<T: ArgumentEncoder>(
    mode: CanisterInstallMode,
    canister_pid: Principal,
    wasm: &[u8],
    args: T,
) -> Result<(), Error> {
    infra::ic::mgmt::install_code(mode, canister_pid, wasm, args).await?;

    let metric_kind = match mode {
        CanisterInstallMode::Install => SystemMetricKind::InstallCode,
        CanisterInstallMode::Reinstall => SystemMetricKind::ReinstallCode,
        CanisterInstallMode::Upgrade(_) => SystemMetricKind::UpgradeCode,
    };
    record_system_metric(metric_kind);

    Ok(())
}

/// Upgrades a canister to the provided wasm.
pub async fn upgrade_canister(canister_pid: Principal, wasm: &[u8]) -> Result<(), Error> {
    infra::ic::mgmt::upgrade_canister(canister_pid, wasm).await?;

    record_system_metric(SystemMetricKind::UpgradeCode);

    #[allow(clippy::cast_precision_loss)]
    let bytes_kb = wasm.len() as f64 / 1_000.0;
    log!(
        Topic::CanisterLifecycle,
        Ok,
        "canister_upgrade: {canister_pid} ({bytes_kb} KB) upgraded"
    );

    Ok(())
}

/// Uninstalls code from a canister and records metrics.
pub async fn uninstall_code(canister_pid: Principal) -> Result<(), Error> {
    infra::ic::mgmt::uninstall_code(canister_pid).await?;

    record_system_metric(SystemMetricKind::UninstallCode);

    log!(
        Topic::CanisterLifecycle,
        Ok,
        "🗑️ uninstall_code: {canister_pid}"
    );

    Ok(())
}

/// Deletes a canister (code + controllers) via the management canister.
pub async fn delete_canister(canister_pid: Principal) -> Result<(), Error> {
    infra::ic::mgmt::delete_canister(canister_pid).await?;

    record_system_metric(SystemMetricKind::DeleteCanister);

    Ok(())
}

//
// ─────────────────────────────── SETTINGS API ────────────────────────────────
//

/// Updates canister settings via the management canister and records metrics.
pub async fn update_settings(args: &UpdateSettingsArgs) -> Result<(), Error> {
    infra::ic::mgmt::update_settings(args).await?;

    record_system_metric(SystemMetricKind::UpdateSettings);

    Ok(())
}
