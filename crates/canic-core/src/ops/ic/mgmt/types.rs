use crate::{
    cdk,
    infra::ic::mgmt::{
        InfraCanisterInstallMode, InfraCanisterSettings, InfraCanisterSnapshot,
        InfraCanisterStatusResult, InfraCanisterStatusType, InfraDefiniteCanisterSettings,
        InfraEnvironmentVariable, InfraLogVisibility, InfraMemoryMetrics, InfraQueryStats,
        InfraUpdateSettingsArgs, InfraUpgradeFlags, InfraWasmMemoryPersistence,
    },
    ops::prelude::*,
};
use candid::Nat;

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
/// CanisterSnapshot
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanisterSnapshot {
    pub id: Vec<u8>,
    pub taken_at_timestamp: u64,
    pub total_size: u64,
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

pub(super) fn canister_status_from_infra(status: InfraCanisterStatusResult) -> CanisterStatus {
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

pub(super) fn canister_snapshot_from_infra(snapshot: InfraCanisterSnapshot) -> CanisterSnapshot {
    CanisterSnapshot {
        id: snapshot.id,
        taken_at_timestamp: snapshot.taken_at_timestamp,
        total_size: snapshot.total_size,
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

pub(super) fn install_mode_to_infra(mode: CanisterInstallMode) -> InfraCanisterInstallMode {
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
        wasm_memory_persistence: Option::<InfraWasmMemoryPersistence>::None,
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

pub(super) fn update_settings_to_infra(args: &UpdateSettingsArgs) -> InfraUpdateSettingsArgs {
    InfraUpdateSettingsArgs {
        canister_id: args.canister_id,
        settings: settings_to_infra(&args.settings),
        sender_canister_version: Some(cdk::api::canister_version()),
    }
}
