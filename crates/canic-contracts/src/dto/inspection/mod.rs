//! Complete native management observations; external IC transport remains Candid.
//! Unsigned evidence uses minimal big-endian magnitudes, including values above u128.
use crate::{dto::canister::EnvironmentVariable, values::canister::CanisterStatusType};
use ic_platform::Principal;

/// No individual magnitude can exceed the existing four-MiB host reply ceiling.
/// Operation-wide frame and allocation limits independently bound the complete graph.
pub const MAGNITUDE_BYTES: usize = 4 * 1024 * 1024;

/// Complete CanisterStatusResponse evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Status {
    pub version: u64,
    pub status: CanisterStatusType,
    pub settings: Settings,
    pub module_hash: Option<Vec<u8>>,
    pub memory_size: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub memory_metrics: MemoryMetrics,
    pub cycles: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub reserved_cycles: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub idle_cycles_burned_per_day: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub query_stats: QueryStats,
}

/// Complete CanisterSettings evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Settings {
    pub controllers: Vec<Principal>,
    pub compute_allocation: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub memory_allocation: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub freezing_threshold: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub reserved_cycles_limit: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub log_visibility: LogVisibility,
    pub log_memory_limit: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub wasm_memory_limit: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub wasm_memory_threshold: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub environment_variables: Vec<EnvironmentVariable>,
}

/// Complete MemoryMetrics evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct MemoryMetrics {
    pub wasm_memory_size: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub stable_memory_size: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub global_memory_size: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub wasm_binary_size: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub custom_sections_size: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub canister_history_size: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub wasm_chunk_store_size: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub snapshots_size: ic_wire::Bytes<MAGNITUDE_BYTES>,
}

/// Complete QueryStats evidence.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct QueryStats {
    pub num_calls_total: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub num_instructions_total: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub request_payload_bytes_total: ic_wire::Bytes<MAGNITUDE_BYTES>,
    pub response_payload_bytes_total: ic_wire::Bytes<MAGNITUDE_BYTES>,
}

/// Native viewer identities retain their external management meaning.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum LogVisibility {
    Controllers,
    Public,
    AllowedViewers(Vec<Principal>),
}

/// Exact controller-selected inspection target.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Request {
    pub canister_id: Principal,
}

/// Status and protected reserve refusal remain independent typed outcomes.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum Response {
    InspectCanister(Box<Status>),
    InspectionReserveRequired(crate::dto::inspection_reserve::InspectionReserve),
}
