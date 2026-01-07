//! ops::ic::mgmt
//!
//! Ops-level wrappers over IC management canister calls.
//! Adds metrics, logging, and normalizes errors into `Error`.

use crate::{
    Error,
    cdk::{
        self,
        mgmt::{
            CanisterStatusResult, CanisterStatusType, DefiniteCanisterSettings,
            EnvironmentVariable as CdkEnvironmentVariable, LogVisibility as CdkLogVisibility,
            MemoryMetrics, QueryStats,
        },
    },
    dto::{
        abi::v1::CanisterInitPayload,
        canister::{
            CanisterSettingsView, CanisterStatusTypeView, CanisterStatusView,
            EnvironmentVariableView, LogVisibilityView, MemoryMetricsView, QueryStatsView,
        },
    },
    infra::ic::mgmt::MgmtInfra,
    ops::{
        ic::IcOpsError,
        prelude::*,
        runtime::metrics::system::{SystemMetricKind, SystemMetrics},
    },
};
use candid::{Nat, utils::ArgumentEncoder};

///
/// CanisterInstallMode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(dead_code)]
pub enum CanisterInstallMode {
    Install,
    Reinstall,
    Upgrade(Option<UpgradeFlags>),
}

///
/// UpgradeFlags
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UpgradeFlags {
    pub skip_pre_upgrade: Option<bool>,
}

///
/// LogVisibility
/// If a type exists to represent a foreign contract or infra boundary,
/// dead-code warnings on its variants are acceptable.
///

#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum LogVisibility {
    Controllers,
    Public,
    AllowedViewers(Vec<Principal>),
}

///
/// EnvironmentVariable
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
}

///
/// CanisterSettings
///

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CanisterSettings {
    pub controllers: Option<Vec<Principal>>,
    pub compute_allocation: Option<Nat>,
    pub memory_allocation: Option<Nat>,
    pub freezing_threshold: Option<Nat>,
    pub reserved_cycles_limit: Option<Nat>,
    pub log_visibility: Option<LogVisibility>,
    pub wasm_memory_limit: Option<Nat>,
    pub wasm_memory_threshold: Option<Nat>,
    pub environment_variables: Option<Vec<EnvironmentVariable>>,
}

///
/// UpdateSettingsArgs
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateSettingsArgs {
    pub canister_id: Principal,
    pub settings: CanisterSettings,
    pub sender_canister_version: Option<u64>,
}

///
/// MgmtOps
///

pub struct MgmtOps;

impl MgmtOps {
    /// Create a canister with explicit controllers and an initial cycle balance.
    pub async fn create_canister(
        controllers: Vec<Principal>,
        cycles: Cycles,
    ) -> Result<Principal, Error> {
        let cycles_snapshot = cycles.clone();
        let pid = MgmtInfra::create_canister(controllers, cycles)
            .await
            .map_err(IcOpsError::from)?;

        SystemMetrics::increment(SystemMetricKind::CreateCanister);
        log!(
            Topic::CanisterLifecycle,
            Ok,
            "canister_create: {pid} cycles={cycles_snapshot}"
        );

        Ok(pid)
    }

    /// Internal ops entrypoint used by workflow and other ops helpers.
    pub async fn canister_status(canister_pid: Principal) -> Result<CanisterStatusView, Error> {
        let status = MgmtInfra::canister_status(canister_pid)
            .await
            .map_err(IcOpsError::from)?;

        SystemMetrics::increment(SystemMetricKind::CanisterStatus);

        Ok(canister_status_to_view(status))
    }

    //
    // ──────────────────────────────── CYCLES API ─────────────────────────────────
    //

    /// Returns the local canister's cycle balance (cheap).
    #[must_use]
    pub fn canister_cycle_balance() -> Cycles {
        MgmtInfra::canister_cycle_balance()
    }

    /// Deposits cycles into a canister and records metrics.
    pub async fn deposit_cycles(canister_pid: Principal, cycles: u128) -> Result<(), Error> {
        MgmtInfra::deposit_cycles(canister_pid, cycles)
            .await
            .map_err(IcOpsError::from)?;

        SystemMetrics::increment(SystemMetricKind::DepositCycles);

        Ok(())
    }

    /// Gets a canister's cycle balance (expensive: calls mgmt canister).
    pub async fn get_cycles(canister_pid: Principal) -> Result<Cycles, Error> {
        let cycles = MgmtInfra::get_cycles(canister_pid)
            .await
            .map_err(IcOpsError::from)?;

        Ok(cycles)
    }

    //
    // ────────────────────────────── INSTALL / UNINSTALL ──────────────────────────
    //

    /// Install or reinstall a *Canic-style* canister using the standard
    /// `(CanisterInitPayload, Option<Vec<u8>>)` argument convention.
    pub async fn install_canister_with_payload(
        mode: CanisterInstallMode,
        canister_pid: Principal,
        wasm: &[u8],
        payload: CanisterInitPayload,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<(), Error> {
        Self::install_code(mode, canister_pid, wasm, (payload, extra_arg)).await
    }

    /// Installs or upgrades a canister with the given wasm + args and records metrics.
    pub async fn install_code<T: ArgumentEncoder>(
        mode: CanisterInstallMode,
        canister_pid: Principal,
        wasm: &[u8],
        args: T,
    ) -> Result<(), Error> {
        let cdk_mode = install_mode_to_cdk(mode);
        MgmtInfra::install_code(cdk_mode, canister_pid, wasm, args)
            .await
            .map_err(IcOpsError::from)?;

        let metric_kind = match mode {
            CanisterInstallMode::Install => SystemMetricKind::InstallCode,
            CanisterInstallMode::Reinstall => SystemMetricKind::ReinstallCode,
            CanisterInstallMode::Upgrade(_) => SystemMetricKind::UpgradeCode,
        };
        SystemMetrics::increment(metric_kind);

        #[allow(clippy::cast_precision_loss)]
        let bytes_kb = wasm.len() as f64 / 1_000.0;
        log!(
            Topic::CanisterLifecycle,
            Ok,
            "install_code: {canister_pid} mode={mode:?} ({bytes_kb} KB)"
        );

        Ok(())
    }

    /// Upgrades a canister to the provided wasm.
    pub async fn upgrade_canister(canister_pid: Principal, wasm: &[u8]) -> Result<(), Error> {
        MgmtInfra::upgrade_canister(canister_pid, wasm)
            .await
            .map_err(IcOpsError::from)?;

        SystemMetrics::increment(SystemMetricKind::UpgradeCode);

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
        MgmtInfra::uninstall_code(canister_pid)
            .await
            .map_err(IcOpsError::from)?;

        SystemMetrics::increment(SystemMetricKind::UninstallCode);

        log!(
            Topic::CanisterLifecycle,
            Ok,
            "🗑️ uninstall_code: {canister_pid}"
        );

        Ok(())
    }

    /// Deletes a canister (code + controllers) via the management canister.
    pub async fn delete_canister(canister_pid: Principal) -> Result<(), Error> {
        MgmtInfra::delete_canister(canister_pid)
            .await
            .map_err(IcOpsError::from)?;

        SystemMetrics::increment(SystemMetricKind::DeleteCanister);

        Ok(())
    }

    //
    // ──────────────────────────────── RANDOMNESS ────────────────────────────────
    //

    /// Query the management canister for raw randomness and record metrics.
    pub async fn raw_rand() -> Result<[u8; 32], Error> {
        let seed = MgmtInfra::raw_rand().await.map_err(IcOpsError::from)?;

        SystemMetrics::increment(SystemMetricKind::RawRand);

        Ok(seed)
    }

    //
    // ─────────────────────────────── SETTINGS API ────────────────────────────────
    //

    /// Updates canister settings via the management canister and records metrics.
    pub async fn update_settings(args: &UpdateSettingsArgs) -> Result<(), Error> {
        let cdk_args = update_settings_to_cdk(args);
        MgmtInfra::update_settings(&cdk_args)
            .await
            .map_err(IcOpsError::from)?;

        SystemMetrics::increment(SystemMetricKind::UpdateSettings);

        Ok(())
    }
}

///
/// View Adapters
///

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

const fn status_type_to_view(status: CanisterStatusType) -> CanisterStatusTypeView {
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

fn log_visibility_to_view(log_visibility: CdkLogVisibility) -> LogVisibilityView {
    match log_visibility {
        CdkLogVisibility::Controllers => LogVisibilityView::Controllers,
        CdkLogVisibility::Public => LogVisibilityView::Public,
        CdkLogVisibility::AllowedViewers(viewers) => LogVisibilityView::AllowedViewers(viewers),
    }
}

fn environment_variable_to_view(variable: CdkEnvironmentVariable) -> EnvironmentVariableView {
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

// --- Ops → CDK adapters -------------------------------------------------

fn install_mode_to_cdk(mode: CanisterInstallMode) -> cdk::mgmt::CanisterInstallMode {
    match mode {
        CanisterInstallMode::Install => cdk::mgmt::CanisterInstallMode::Install,
        CanisterInstallMode::Reinstall => cdk::mgmt::CanisterInstallMode::Reinstall,
        CanisterInstallMode::Upgrade(flags) => {
            cdk::mgmt::CanisterInstallMode::Upgrade(flags.map(upgrade_flags_to_cdk))
        }
    }
}

const fn upgrade_flags_to_cdk(flags: UpgradeFlags) -> cdk::mgmt::UpgradeFlags {
    cdk::mgmt::UpgradeFlags {
        skip_pre_upgrade: flags.skip_pre_upgrade,
        wasm_memory_persistence: None,
    }
}

fn settings_to_cdk(settings: &CanisterSettings) -> cdk::mgmt::CanisterSettings {
    cdk::mgmt::CanisterSettings {
        controllers: settings.controllers.clone(),
        compute_allocation: settings.compute_allocation.clone(),
        memory_allocation: settings.memory_allocation.clone(),
        freezing_threshold: settings.freezing_threshold.clone(),
        reserved_cycles_limit: settings.reserved_cycles_limit.clone(),
        log_visibility: settings.log_visibility.clone().map(log_visibility_to_cdk),
        wasm_memory_limit: settings.wasm_memory_limit.clone(),
        wasm_memory_threshold: settings.wasm_memory_threshold.clone(),
        environment_variables: settings
            .environment_variables
            .clone()
            .map(|vars| vars.into_iter().map(environment_variable_to_cdk).collect()),
    }
}

fn log_visibility_to_cdk(setting: LogVisibility) -> cdk::mgmt::LogVisibility {
    match setting {
        LogVisibility::Controllers => cdk::mgmt::LogVisibility::Controllers,
        LogVisibility::Public => cdk::mgmt::LogVisibility::Public,
        LogVisibility::AllowedViewers(viewers) => cdk::mgmt::LogVisibility::AllowedViewers(viewers),
    }
}

fn environment_variable_to_cdk(variable: EnvironmentVariable) -> cdk::mgmt::EnvironmentVariable {
    cdk::mgmt::EnvironmentVariable {
        name: variable.name,
        value: variable.value,
    }
}

fn update_settings_to_cdk(args: &UpdateSettingsArgs) -> cdk::mgmt::UpdateSettingsArgs {
    cdk::mgmt::UpdateSettingsArgs {
        canister_id: args.canister_id,
        settings: settings_to_cdk(&args.settings),
    }
}
