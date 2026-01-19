use crate::dto::prelude::*;

///
/// CanisterInfo
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterInfo {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
    pub created_at: u64,
}

///
/// CanisterStatusResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterStatusResponse {
    pub status: CanisterStatusType,
    pub settings: CanisterSettings,
    pub module_hash: Option<Vec<u8>>,
    pub memory_size: Nat,
    pub memory_metrics: MemoryMetrics,
    pub cycles: Nat,
    pub reserved_cycles: Nat,
    pub idle_cycles_burned_per_day: Nat,
    pub query_stats: QueryStats,
}

///
/// CanisterStatusType
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum CanisterStatusType {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "stopped")]
    Stopped,
}

///
/// CanisterSettings
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterSettings {
    pub controllers: Vec<Principal>,
    pub compute_allocation: Nat,
    pub memory_allocation: Nat,
    pub freezing_threshold: Nat,
    pub reserved_cycles_limit: Nat,
    pub log_visibility: LogVisibility,
    pub wasm_memory_limit: Nat,
    pub wasm_memory_threshold: Nat,
    pub environment_variables: Vec<EnvironmentVariable>,
}

///
/// LogVisibility
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum LogVisibility {
    #[serde(rename = "controllers")]
    Controllers,
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "allowed_viewers")]
    AllowedViewers(Vec<Principal>),
}

///
/// EnvironmentVariable
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
}

///
/// MemoryMetrics
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct MemoryMetrics {
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
/// QueryStats
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct QueryStats {
    pub num_calls_total: Nat,
    pub num_instructions_total: Nat,
    pub request_payload_bytes_total: Nat,
    pub response_payload_bytes_total: Nat,
}
