//! ops::ic::mgmt
//!
//! Ops-level wrappers over IC management canister calls.
//! Adds metrics, logging, and normalizes errors into `InternalError`.

use crate::{
    InternalError, cdk,
    ids::SystemMetricKind,
    infra::ic::mgmt::{
        InfraCanisterInstallMode, InfraCanisterSettings, InfraCanisterStatusResult,
        InfraCanisterStatusType, InfraDefiniteCanisterSettings, InfraEnvironmentVariable,
        InfraLogVisibility, InfraMemoryMetrics, InfraQueryStats, InfraUpdateSettingsArgs,
        InfraUpgradeFlags, MgmtInfra,
    },
    ops::{ic::IcOpsError, prelude::*, runtime::metrics::system::SystemMetrics},
};
use candid::{Nat, utils::ArgumentEncoder};

///
/// CanisterInstallMode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    pub log_memory_limit: Option<Nat>,
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
/// CanisterStatus
///

#[derive(Clone, Debug)]
pub struct CanisterStatus {
    pub status: CanisterStatusType,
    pub settings: CanisterSettingsSnapshot,
    pub module_hash: Option<Vec<u8>>,
    pub memory_size: Nat,
    pub memory_metrics: MemoryMetricsSnapshot,
    pub cycles: Nat,
    pub reserved_cycles: Nat,
    pub idle_cycles_burned_per_day: Nat,
    pub query_stats: QueryStatsSnapshot,
}

///
/// CanisterStatusType
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanisterStatusType {
    Running,
    Stopping,
    Stopped,
}

///
/// CanisterSettingsSnapshot
///

#[derive(Clone, Debug)]
pub struct CanisterSettingsSnapshot {
    pub controllers: Vec<Principal>,
    pub compute_allocation: Nat,
    pub memory_allocation: Nat,
    pub freezing_threshold: Nat,
    pub reserved_cycles_limit: Nat,
    pub log_visibility: LogVisibility,
    pub log_memory_limit: Nat,
    pub wasm_memory_limit: Nat,
    pub wasm_memory_threshold: Nat,
    pub environment_variables: Vec<EnvironmentVariable>,
}

///
/// MemoryMetricsSnapshot
///

#[derive(Clone, Debug)]
pub struct MemoryMetricsSnapshot {
    pub wasm_memory_size: Nat,
    pub stable_memory_size: Nat,
    pub global_memory_size: Nat,
    pub wasm_binary_size: Nat,
    pub custom_sections_size: Nat,
    pub canister_history_size: Nat,
    pub wasm_chunk_store_size: Nat,
    pub snapshots_size: Nat,
}

///
/// QueryStatsSnapshot
///

#[derive(Clone, Debug)]
pub struct QueryStatsSnapshot {
    pub num_calls_total: Nat,
    pub num_instructions_total: Nat,
    pub request_payload_bytes_total: Nat,
    pub response_payload_bytes_total: Nat,
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
    ) -> Result<Principal, InternalError> {
        let pid = MgmtInfra::create_canister(controllers, cycles)
            .await
            .map_err(IcOpsError::from)?;

        SystemMetrics::increment(SystemMetricKind::CreateCanister);

        Ok(pid)
    }

    /// Internal ops entrypoint used by workflow and other ops helpers.
    pub async fn canister_status(canister_pid: Principal) -> Result<CanisterStatus, InternalError> {
        let status = MgmtInfra::canister_status(canister_pid)
            .await
            .map_err(IcOpsError::from)?;

        SystemMetrics::increment(SystemMetricKind::CanisterStatus);

        Ok(canister_status_from_infra(status))
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
    pub async fn deposit_cycles(
        canister_pid: Principal,
        cycles: u128,
    ) -> Result<(), InternalError> {
        MgmtInfra::deposit_cycles(canister_pid, cycles)
            .await
            .map_err(IcOpsError::from)?;

        SystemMetrics::increment(SystemMetricKind::DepositCycles);

        Ok(())
    }

    /// Gets a canister's cycle balance (expensive: calls mgmt canister).
    pub async fn get_cycles(canister_pid: Principal) -> Result<Cycles, InternalError> {
        let cycles = MgmtInfra::get_cycles(canister_pid)
            .await
            .map_err(IcOpsError::from)?;

        Ok(cycles)
    }

    //
    // ────────────────────────────── INSTALL / UNINSTALL ──────────────────────────
    //

    /// Install or upgrade a canister from chunks stored in one same-subnet store canister.
    pub async fn install_chunked_code<T: ArgumentEncoder>(
        mode: CanisterInstallMode,
        target_canister: Principal,
        store_canister: Principal,
        chunk_hashes_list: Vec<Vec<u8>>,
        wasm_module_hash: Vec<u8>,
        args: T,
    ) -> Result<(), InternalError> {
        let chunk_count = chunk_hashes_list.len();
        MgmtInfra::install_chunked_code(
            install_mode_to_infra(mode),
            target_canister,
            store_canister,
            chunk_hashes_list,
            wasm_module_hash,
            args,
        )
        .await
        .map_err(IcOpsError::from)?;

        let metric_kind = match mode {
            CanisterInstallMode::Install => SystemMetricKind::InstallCode,
            CanisterInstallMode::Reinstall => SystemMetricKind::ReinstallCode,
            CanisterInstallMode::Upgrade(_) => SystemMetricKind::UpgradeCode,
        };
        SystemMetrics::increment(metric_kind);

        log!(
            Topic::CanisterLifecycle,
            Ok,
            "install_chunked_code: {target_canister} mode={mode:?} store={store_canister} chunks={chunk_count}"
        );

        Ok(())
    }

    /// Install or upgrade a canister from an embedded wasm payload.
    pub async fn install_code<T: ArgumentEncoder>(
        mode: CanisterInstallMode,
        target_canister: Principal,
        wasm_module: Vec<u8>,
        args: T,
    ) -> Result<(), InternalError> {
        let payload_size_bytes = wasm_module.len();
        MgmtInfra::install_code(
            install_mode_to_infra(mode),
            target_canister,
            wasm_module,
            args,
        )
        .await
        .map_err(IcOpsError::from)?;

        let metric_kind = match mode {
            CanisterInstallMode::Install => SystemMetricKind::InstallCode,
            CanisterInstallMode::Reinstall => SystemMetricKind::ReinstallCode,
            CanisterInstallMode::Upgrade(_) => SystemMetricKind::UpgradeCode,
        };
        SystemMetrics::increment(metric_kind);

        log!(
            Topic::CanisterLifecycle,
            Ok,
            "install_code: {target_canister} mode={mode:?} embedded_bytes={payload_size_bytes}"
        );

        Ok(())
    }

    /// Install or reinstall a Canic-style canister from chunk-store-backed wasm.
    pub async fn install_chunked_canister_with_payload<P: CandidType>(
        mode: CanisterInstallMode,
        target_canister: Principal,
        store_canister: Principal,
        chunk_hashes_list: Vec<Vec<u8>>,
        wasm_module_hash: Vec<u8>,
        payload: P,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<(), InternalError> {
        Self::install_chunked_code(
            mode,
            target_canister,
            store_canister,
            chunk_hashes_list,
            wasm_module_hash,
            (payload, extra_arg),
        )
        .await
    }

    /// Install or reinstall a Canic-style canister from an embedded wasm payload.
    pub async fn install_embedded_canister_with_payload<P: CandidType>(
        mode: CanisterInstallMode,
        target_canister: Principal,
        wasm_module: Vec<u8>,
        payload: P,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<(), InternalError> {
        Self::install_code(mode, target_canister, wasm_module, (payload, extra_arg)).await
    }

    /// Upload one wasm chunk into a canister's chunk store.
    pub async fn upload_chunk(
        canister_pid: Principal,
        chunk: Vec<u8>,
    ) -> Result<Vec<u8>, InternalError> {
        let chunk_len = chunk.len();
        let hash = MgmtInfra::upload_chunk(canister_pid, chunk)
            .await
            .map_err(IcOpsError::from)?;

        #[expect(clippy::cast_precision_loss)]
        let bytes_kb = chunk_len as f64 / 1_000.0;
        log!(
            Topic::CanisterLifecycle,
            Ok,
            "upload_chunk: {canister_pid} ({bytes_kb} KB)"
        );

        Ok(hash)
    }

    /// List the chunk hashes currently stored in one canister's chunk store.
    pub async fn stored_chunks(canister_pid: Principal) -> Result<Vec<Vec<u8>>, InternalError> {
        Ok(MgmtInfra::stored_chunks(canister_pid)
            .await
            .map_err(IcOpsError::from)?)
    }

    /// Clear the chunk store of one canister.
    pub async fn clear_chunk_store(canister_pid: Principal) -> Result<(), InternalError> {
        MgmtInfra::clear_chunk_store(canister_pid)
            .await
            .map_err(IcOpsError::from)?;

        log!(
            Topic::CanisterLifecycle,
            Ok,
            "clear_chunk_store: {canister_pid}"
        );

        Ok(())
    }

    /// Uninstalls code from a canister and records metrics.
    pub async fn uninstall_code(canister_pid: Principal) -> Result<(), InternalError> {
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

    /// Stops a canister via the management canister.
    pub async fn stop_canister(canister_pid: Principal) -> Result<(), InternalError> {
        MgmtInfra::stop_canister(canister_pid)
            .await
            .map_err(IcOpsError::from)?;

        log!(
            Topic::CanisterLifecycle,
            Ok,
            "stop_canister: {canister_pid}"
        );

        Ok(())
    }

    /// Deletes a canister (code + controllers) via the management canister.
    pub async fn delete_canister(canister_pid: Principal) -> Result<(), InternalError> {
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
    pub async fn raw_rand() -> Result<[u8; 32], InternalError> {
        let seed = MgmtInfra::raw_rand().await.map_err(IcOpsError::from)?;

        SystemMetrics::increment(SystemMetricKind::RawRand);

        Ok(seed)
    }

    //
    // ─────────────────────────────── SETTINGS API ────────────────────────────────
    //

    /// Updates canister settings via the management canister and records metrics.
    pub async fn update_settings(args: &UpdateSettingsArgs) -> Result<(), InternalError> {
        let infra_args = update_settings_to_infra(args);
        MgmtInfra::update_settings(&infra_args)
            .await
            .map_err(IcOpsError::from)?;

        SystemMetrics::increment(SystemMetricKind::UpdateSettings);

        Ok(())
    }
}

///
/// Infra Adapters
///

fn canister_status_from_infra(status: InfraCanisterStatusResult) -> CanisterStatus {
    CanisterStatus {
        status: status_type_from_infra(status.status),
        settings: settings_from_infra(status.settings),
        module_hash: status.module_hash,
        memory_size: status.memory_size,
        memory_metrics: memory_metrics_from_infra(status.memory_metrics),
        cycles: status.cycles,
        reserved_cycles: status.reserved_cycles,
        idle_cycles_burned_per_day: status.idle_cycles_burned_per_day,
        query_stats: query_stats_from_infra(status.query_stats),
    }
}

const fn status_type_from_infra(status: InfraCanisterStatusType) -> CanisterStatusType {
    match status {
        InfraCanisterStatusType::Running => CanisterStatusType::Running,
        InfraCanisterStatusType::Stopping => CanisterStatusType::Stopping,
        InfraCanisterStatusType::Stopped => CanisterStatusType::Stopped,
    }
}

fn settings_from_infra(settings: InfraDefiniteCanisterSettings) -> CanisterSettingsSnapshot {
    CanisterSettingsSnapshot {
        controllers: settings.controllers,
        compute_allocation: settings.compute_allocation,
        memory_allocation: settings.memory_allocation,
        freezing_threshold: settings.freezing_threshold,
        reserved_cycles_limit: settings.reserved_cycles_limit,
        log_visibility: log_visibility_from_infra(settings.log_visibility),
        log_memory_limit: settings.log_memory_limit,
        wasm_memory_limit: settings.wasm_memory_limit,
        wasm_memory_threshold: settings.wasm_memory_threshold,
        environment_variables: settings
            .environment_variables
            .into_iter()
            .map(environment_variable_from_infra)
            .collect(),
    }
}

fn log_visibility_from_infra(log_visibility: InfraLogVisibility) -> LogVisibility {
    match log_visibility {
        InfraLogVisibility::Controllers => LogVisibility::Controllers,
        InfraLogVisibility::Public => LogVisibility::Public,
        InfraLogVisibility::AllowedViewers(viewers) => LogVisibility::AllowedViewers(viewers),
    }
}

fn environment_variable_from_infra(variable: InfraEnvironmentVariable) -> EnvironmentVariable {
    EnvironmentVariable {
        name: variable.name,
        value: variable.value,
    }
}

fn memory_metrics_from_infra(metrics: InfraMemoryMetrics) -> MemoryMetricsSnapshot {
    MemoryMetricsSnapshot {
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

fn query_stats_from_infra(stats: InfraQueryStats) -> QueryStatsSnapshot {
    QueryStatsSnapshot {
        num_calls_total: stats.num_calls_total,
        num_instructions_total: stats.num_instructions_total,
        request_payload_bytes_total: stats.request_payload_bytes_total,
        response_payload_bytes_total: stats.response_payload_bytes_total,
    }
}

// --- Ops → Infra adapters -----------------------------------------------

fn install_mode_to_infra(mode: CanisterInstallMode) -> InfraCanisterInstallMode {
    match mode {
        CanisterInstallMode::Install => InfraCanisterInstallMode::Install,
        CanisterInstallMode::Reinstall => InfraCanisterInstallMode::Reinstall,
        CanisterInstallMode::Upgrade(flags) => {
            InfraCanisterInstallMode::Upgrade(flags.map(upgrade_flags_to_infra))
        }
    }
}

const fn upgrade_flags_to_infra(flags: UpgradeFlags) -> InfraUpgradeFlags {
    InfraUpgradeFlags {
        skip_pre_upgrade: flags.skip_pre_upgrade,
        wasm_memory_persistence: None,
    }
}

fn settings_to_infra(settings: &CanisterSettings) -> InfraCanisterSettings {
    InfraCanisterSettings {
        controllers: settings.controllers.clone(),
        compute_allocation: settings.compute_allocation.clone(),
        memory_allocation: settings.memory_allocation.clone(),
        freezing_threshold: settings.freezing_threshold.clone(),
        reserved_cycles_limit: settings.reserved_cycles_limit.clone(),
        log_visibility: settings.log_visibility.clone().map(log_visibility_to_infra),
        log_memory_limit: settings.log_memory_limit.clone(),
        wasm_memory_limit: settings.wasm_memory_limit.clone(),
        wasm_memory_threshold: settings.wasm_memory_threshold.clone(),
        environment_variables: settings.environment_variables.clone().map(|vars| {
            vars.into_iter()
                .map(environment_variable_to_infra)
                .collect()
        }),
    }
}

fn log_visibility_to_infra(setting: LogVisibility) -> InfraLogVisibility {
    match setting {
        LogVisibility::Controllers => InfraLogVisibility::Controllers,
        LogVisibility::Public => InfraLogVisibility::Public,
        LogVisibility::AllowedViewers(viewers) => InfraLogVisibility::AllowedViewers(viewers),
    }
}

fn environment_variable_to_infra(variable: EnvironmentVariable) -> InfraEnvironmentVariable {
    InfraEnvironmentVariable {
        name: variable.name,
        value: variable.value,
    }
}

fn update_settings_to_infra(args: &UpdateSettingsArgs) -> InfraUpdateSettingsArgs {
    InfraUpdateSettingsArgs {
        canister_id: args.canister_id,
        settings: settings_to_infra(&args.settings),
        sender_canister_version: Some(cdk::api::canister_version()),
    }
}
