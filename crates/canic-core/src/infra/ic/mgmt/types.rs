//! Module: infra::ic::mgmt::types
//!
//! Responsibility: define Candid payload shapes for management canister calls.
//! Does not own: lifecycle orchestration, deployment policy, or public DTO schemas.
//! Boundary: management infra leaves encode and decode these raw payloads.

use crate::cdk::candid::{CandidType, Nat, Principal};
use serde::Deserialize;

//
// InfraCanisterSettings
//

#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct InfraCanisterSettings {
    pub controllers: Option<Vec<Principal>>,
    pub compute_allocation: Option<Nat>,
    pub memory_allocation: Option<Nat>,
    pub freezing_threshold: Option<Nat>,
    pub reserved_cycles_limit: Option<Nat>,
    pub log_visibility: Option<InfraLogVisibility>,
    pub log_memory_limit: Option<Nat>,
    pub wasm_memory_limit: Option<Nat>,
    pub wasm_memory_threshold: Option<Nat>,
    pub environment_variables: Option<Vec<InfraEnvironmentVariable>>,
}

//
// InfraCreateCanisterArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub(super) struct InfraCreateCanisterArgs {
    pub(super) settings: Option<InfraCanisterSettings>,
    pub(super) sender_canister_version: Option<u64>,
}

//
// InfraCreateCanisterResult
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub(super) struct InfraCreateCanisterResult {
    pub(super) canister_id: Principal,
}

//
// InfraCanisterIdRecord
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub(super) struct InfraCanisterIdRecord {
    pub(super) canister_id: Principal,
}

//
// InfraCanisterIdRecordExtended
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub(super) struct InfraCanisterIdRecordExtended {
    pub(super) canister_id: Principal,
    pub(super) sender_canister_version: Option<u64>,
}

//
// InfraCanisterInstallMode
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum InfraCanisterInstallMode {
    #[serde(rename = "install")]
    Install,
    #[serde(rename = "reinstall")]
    Reinstall,
    #[serde(rename = "upgrade")]
    Upgrade(Option<InfraUpgradeFlags>),
}

//
// InfraUpgradeFlags
//

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct InfraUpgradeFlags {
    pub skip_pre_upgrade: Option<bool>,
    pub wasm_memory_persistence: Option<InfraWasmMemoryPersistence>,
}

//
// InfraWasmMemoryPersistence
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum InfraWasmMemoryPersistence {
    #[serde(rename = "keep")]
    Keep,
    #[serde(rename = "replace")]
    Replace,
}

//
// InfraChunkHash
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub(super) struct InfraChunkHash {
    pub(super) hash: Vec<u8>,
}

//
// InfraUploadChunkArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub(super) struct InfraUploadChunkArgs {
    pub(super) canister_id: Principal,
    pub(super) chunk: Vec<u8>,
}

//
// InfraClearChunkStoreArgs
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub(super) struct InfraClearChunkStoreArgs {
    pub(super) canister_id: Principal,
}

//
// InfraInstallChunkedCodeArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub(super) struct InfraInstallChunkedCodeArgs {
    pub(super) mode: InfraCanisterInstallMode,
    pub(super) target_canister: Principal,
    pub(super) store_canister: Option<Principal>,
    pub(super) chunk_hashes_list: Vec<InfraChunkHash>,
    pub(super) wasm_module_hash: Vec<u8>,
    pub(super) arg: Vec<u8>,
    pub(super) sender_canister_version: Option<u64>,
}

//
// InfraInstallCodeArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub(super) struct InfraInstallCodeArgs {
    pub(super) mode: InfraCanisterInstallMode,
    pub(super) canister_id: Principal,
    pub(super) wasm_module: Vec<u8>,
    pub(super) arg: Vec<u8>,
    pub(super) sender_canister_version: Option<u64>,
}

//
// InfraUpdateSettingsArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraUpdateSettingsArgs {
    pub canister_id: Principal,
    pub settings: InfraCanisterSettings,
    pub sender_canister_version: Option<u64>,
}

//
// InfraEcdsaCurve
//

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub enum InfraEcdsaCurve {
    #[default]
    #[serde(rename = "secp256k1")]
    Secp256k1,
}

impl From<InfraEcdsaCurve> for u32 {
    fn from(value: InfraEcdsaCurve) -> Self {
        match value {
            InfraEcdsaCurve::Secp256k1 => 0,
        }
    }
}

//
// InfraEcdsaKeyId
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraEcdsaKeyId {
    pub curve: InfraEcdsaCurve,
    pub name: String,
}

//
// InfraEcdsaPublicKeyArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraEcdsaPublicKeyArgs {
    pub canister_id: Option<Principal>,
    pub derivation_path: Vec<Vec<u8>>,
    pub key_id: InfraEcdsaKeyId,
}

//
// InfraEcdsaPublicKeyResult
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraEcdsaPublicKeyResult {
    pub public_key: Vec<u8>,
    pub chain_code: Vec<u8>,
}

//
// InfraSignWithEcdsaArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraSignWithEcdsaArgs {
    pub message_hash: Vec<u8>,
    pub derivation_path: Vec<Vec<u8>>,
    pub key_id: InfraEcdsaKeyId,
}

//
// InfraSignWithEcdsaResult
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraSignWithEcdsaResult {
    pub signature: Vec<u8>,
}

//
// InfraCanisterStatusType
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum InfraCanisterStatusType {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "stopped")]
    Stopped,
}

//
// InfraLogVisibility
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum InfraLogVisibility {
    #[serde(rename = "controllers")]
    Controllers,
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "allowed_viewers")]
    AllowedViewers(Vec<Principal>),
}

//
// InfraEnvironmentVariable
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraEnvironmentVariable {
    pub name: String,
    pub value: String,
}

//
// InfraDefiniteCanisterSettings
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraDefiniteCanisterSettings {
    pub controllers: Vec<Principal>,
    pub compute_allocation: Nat,
    pub memory_allocation: Nat,
    pub freezing_threshold: Nat,
    pub reserved_cycles_limit: Nat,
    pub log_visibility: InfraLogVisibility,
    pub log_memory_limit: Nat,
    pub wasm_memory_limit: Nat,
    pub wasm_memory_threshold: Nat,
    pub environment_variables: Vec<InfraEnvironmentVariable>,
}

//
// InfraMemoryMetrics
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraMemoryMetrics {
    pub wasm_memory_size: Nat,
    pub stable_memory_size: Nat,
    pub global_memory_size: Nat,
    pub wasm_binary_size: Nat,
    pub custom_sections_size: Nat,
    pub canister_history_size: Nat,
    pub wasm_chunk_store_size: Nat,
    pub snapshots_size: Nat,
}

//
// InfraQueryStats
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraQueryStats {
    pub num_calls_total: Nat,
    pub num_instructions_total: Nat,
    pub request_payload_bytes_total: Nat,
    pub response_payload_bytes_total: Nat,
}

//
// InfraCanisterStatusResult
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraCanisterStatusResult {
    pub status: InfraCanisterStatusType,
    pub settings: InfraDefiniteCanisterSettings,
    pub module_hash: Option<Vec<u8>>,
    pub memory_size: Nat,
    pub memory_metrics: InfraMemoryMetrics,
    pub cycles: Nat,
    pub reserved_cycles: Nat,
    pub idle_cycles_burned_per_day: Nat,
    pub query_stats: InfraQueryStats,
}
