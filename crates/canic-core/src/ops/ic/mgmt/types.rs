//! Module: ops::ic::mgmt::types
//!
//! Responsibility: map management infra types into ops-owned boundary shapes.
//! Does not own: management call execution, endpoint DTOs, or lifecycle policy.
//! Boundary: type conversion layer for `ops::ic::mgmt`.

use crate::{
    cdk,
    infra::ic::mgmt::{
        InfraCanisterInstallMode, InfraCanisterSettings, InfraCanisterStatusResult,
        InfraCanisterStatusType, InfraDefiniteCanisterSettings, InfraEcdsaCurve, InfraEcdsaKeyId,
        InfraEcdsaPublicKeyArgs, InfraEcdsaPublicKeyResult, InfraEnvironmentVariable,
        InfraLogVisibility, InfraMemoryMetrics, InfraQueryStats, InfraSignWithEcdsaArgs,
        InfraSignWithEcdsaResult, InfraUpdateSettingsArgs, InfraUpgradeFlags,
        InfraWasmMemoryPersistence,
    },
    ops::prelude::*,
};
use candid::Nat;

pub use crate::domain::canister::{CanisterStatusType, LogVisibility};

///
/// CanisterInstallMode
///
/// Operations-layer install mode for management canister code installation.
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
/// Operations-layer upgrade flags for management canister code installation.
///

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UpgradeFlags {
    pub skip_pre_upgrade: Option<bool>,
}

///
/// EnvironmentVariable
///
/// Operations-layer environment variable setting for canister settings updates.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
}

///
/// CanisterSettings
///
/// Operations-layer canister settings update shape.
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
/// Operations-layer arguments for management canister settings updates.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateSettingsArgs {
    pub canister_id: Principal,
    pub settings: CanisterSettings,
    pub sender_canister_version: Option<u64>,
}

///
/// EcdsaKeyId
///
/// Operations-layer ECDSA key id for management-canister chain-key calls.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcdsaKeyId {
    pub name: String,
}

///
/// EcdsaPublicKeyArgs
///
/// Operations-layer arguments for the management-canister ECDSA public-key API.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcdsaPublicKeyArgs {
    pub canister_id: Option<Principal>,
    pub derivation_path: Vec<Vec<u8>>,
    pub key_id: EcdsaKeyId,
}

///
/// EcdsaPublicKeyResult
///
/// Operations-layer result for the management-canister ECDSA public-key API.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcdsaPublicKeyResult {
    pub public_key: Vec<u8>,
    pub chain_code: Vec<u8>,
}

///
/// SignWithEcdsaArgs
///
/// Operations-layer arguments for the management-canister ECDSA signing API.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignWithEcdsaArgs {
    pub message_hash: [u8; 32],
    pub derivation_path: Vec<Vec<u8>>,
    pub key_id: EcdsaKeyId,
}

///
/// SignWithEcdsaResult
///
/// Operations-layer result for the management-canister ECDSA signing API.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignWithEcdsaResult {
    pub signature: Vec<u8>,
}

///
/// CanisterStatus
///
/// Operations-layer canister status snapshot.
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
/// CanisterSettingsSnapshot
///
/// Operations-layer canister settings snapshot returned by status calls.
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
/// Operations-layer canister memory metrics snapshot returned by status calls.
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
/// Operations-layer canister query stats snapshot returned by status calls.
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

pub(super) fn ecdsa_public_key_args_to_infra(args: &EcdsaPublicKeyArgs) -> InfraEcdsaPublicKeyArgs {
    InfraEcdsaPublicKeyArgs {
        canister_id: args.canister_id,
        derivation_path: args.derivation_path.clone(),
        key_id: ecdsa_key_id_to_infra(&args.key_id),
    }
}

pub(super) fn ecdsa_public_key_from_infra(
    result: InfraEcdsaPublicKeyResult,
) -> EcdsaPublicKeyResult {
    EcdsaPublicKeyResult {
        public_key: result.public_key,
        chain_code: result.chain_code,
    }
}

pub(super) fn sign_with_ecdsa_args_to_infra(args: &SignWithEcdsaArgs) -> InfraSignWithEcdsaArgs {
    InfraSignWithEcdsaArgs {
        message_hash: args.message_hash.to_vec(),
        derivation_path: args.derivation_path.clone(),
        key_id: ecdsa_key_id_to_infra(&args.key_id),
    }
}

pub(super) fn sign_with_ecdsa_from_infra(result: InfraSignWithEcdsaResult) -> SignWithEcdsaResult {
    SignWithEcdsaResult {
        signature: result.signature,
    }
}

fn ecdsa_key_id_to_infra(key_id: &EcdsaKeyId) -> InfraEcdsaKeyId {
    InfraEcdsaKeyId {
        curve: InfraEcdsaCurve::Secp256k1,
        name: key_id.name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_key_id_uses_the_supported_management_curve() {
        let key_id = ecdsa_key_id_to_infra(&EcdsaKeyId {
            name: "key_1".to_string(),
        });

        assert_eq!(key_id.curve, InfraEcdsaCurve::Secp256k1);
        assert_eq!(key_id.name, "key_1");
    }
}
