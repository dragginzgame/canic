//! Module: infra::ic::mgmt::types
//!
//! Responsibility: define Candid payload shapes for management canister calls.
//! Does not own: lifecycle orchestration, deployment policy, or public DTO schemas.
//! Boundary: management infra leaves encode and decode these raw payloads.

use ic_platform::{
    candid::contract::{CandidType, Nat},
    management_canister_types::{CanisterStatusType, ChunkHash, MemoryMetrics, QueryStats},
    principal::ExternalPrincipal as Principal,
};
//
// InfraCanisterSettings
//

use serde::Deserialize;

#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
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
// InfraCanisterIdRecordExtended
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub(super) struct InfraCanisterIdRecordExtended {
    pub(super) canister_id: Principal,
    pub(super) sender_canister_version: Option<u64>,
}

//
// InfraCanisterInstallMode
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub(super) enum InfraCanisterInstallMode {
    #[serde(rename = "install")]
    Install,
}

//
// InfraUploadChunkArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub(super) struct InfraUploadChunkArgs {
    pub(super) canister_id: Principal,
    pub(super) chunk: Vec<u8>,
}

//
// InfraInstallChunkedCodeArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub(super) struct InfraInstallChunkedCodeArgs {
    pub(super) mode: InfraCanisterInstallMode,
    pub(super) target_canister: Principal,
    pub(super) store_canister: Option<Principal>,
    pub(super) chunk_hashes_list: Vec<ChunkHash>,
    pub(super) wasm_module_hash: Vec<u8>,
    pub(super) arg: Vec<u8>,
    pub(super) sender_canister_version: Option<u64>,
}

//
// InfraUpdateSettingsArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct InfraUpdateSettingsArgs {
    pub canister_id: Principal,
    pub settings: InfraCanisterSettings,
    pub sender_canister_version: Option<u64>,
}

//
// InfraEcdsaCurve
//

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
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
#[candid_path("ic_platform::candid::contract")]
pub struct InfraEcdsaKeyId {
    pub curve: InfraEcdsaCurve,
    pub name: String,
}

//
// InfraEcdsaPublicKeyArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct InfraEcdsaPublicKeyArgs {
    pub canister_id: Option<Principal>,
    pub derivation_path: Vec<Vec<u8>>,
    pub key_id: InfraEcdsaKeyId,
}

//
// InfraEcdsaPublicKeyResult
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct InfraEcdsaPublicKeyResult {
    pub public_key: Vec<u8>,
    pub chain_code: Vec<u8>,
}

//
// InfraSignWithEcdsaArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct InfraSignWithEcdsaArgs {
    pub message_hash: Vec<u8>,
    pub derivation_path: Vec<Vec<u8>>,
    pub key_id: InfraEcdsaKeyId,
}

//
// InfraSignWithEcdsaResult
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct InfraSignWithEcdsaResult {
    pub signature: Vec<u8>,
}

//
// InfraLogVisibility
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
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
#[candid_path("ic_platform::candid::contract")]
pub struct InfraEnvironmentVariable {
    pub name: String,
    pub value: String,
}

//
// InfraDefiniteCanisterSettings
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
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
// InfraCanisterStatusResult
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct InfraCanisterStatusResult {
    pub version: u64,
    pub status: CanisterStatusType,
    pub settings: InfraDefiniteCanisterSettings,
    pub module_hash: Option<Vec<u8>>,
    pub memory_size: Nat,
    pub memory_metrics: MemoryMetrics,
    pub cycles: Nat,
    pub reserved_cycles: Nat,
    pub idle_cycles_burned_per_day: Nat,
    pub query_stats: QueryStats,
}
