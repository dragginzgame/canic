use crate::dto::prelude::*;

///
/// CanisterRecordView
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterRecordView {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
    pub created_at: u64,
}

///
/// CanisterStatusView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterStatusView {
    pub status: CanisterStatusTypeView,
    pub settings: CanisterSettingsView,
    pub module_hash: Option<Vec<u8>>,
    pub memory_size: Nat,
    pub memory_metrics: MemoryMetricsView,
    pub cycles: Nat,
    pub reserved_cycles: Nat,
    pub idle_cycles_burned_per_day: Nat,
    pub query_stats: QueryStatsView,
}

///
/// CanisterStatusTypeView
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum CanisterStatusTypeView {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "stopped")]
    Stopped,
}

///
/// CanisterSettingsView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct CanisterSettingsView {
    pub controllers: Vec<Principal>,
    pub compute_allocation: Nat,
    pub memory_allocation: Nat,
    pub freezing_threshold: Nat,
    pub reserved_cycles_limit: Nat,
    pub log_visibility: LogVisibilityView,
    pub wasm_memory_limit: Nat,
    pub wasm_memory_threshold: Nat,
    pub environment_variables: Vec<EnvironmentVariableView>,
}

///
/// LogVisibilityView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum LogVisibilityView {
    #[serde(rename = "controllers")]
    Controllers,
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "allowed_viewers")]
    AllowedViewers(Vec<Principal>),
}

///
/// EnvironmentVariableView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct EnvironmentVariableView {
    pub name: String,
    pub value: String,
}

///
/// MemoryMetricsView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct MemoryMetricsView {
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
/// QueryStatsView
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct QueryStatsView {
    pub num_calls_total: Nat,
    pub num_instructions_total: Nat,
    pub request_payload_bytes_total: Nat,
    pub response_payload_bytes_total: Nat,
}
