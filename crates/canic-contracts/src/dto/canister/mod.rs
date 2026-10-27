use crate::dto::prelude::*;

pub use crate::values::canister::{CanisterStatusType, LogVisibility};

/// Protected reserve sample for an exact target's management status call.
/// A preflight query is indicative; only an attempted call reports SDK admission failure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct CanisterInspectionReserveResponse {
    pub caller: Principal,
    pub canister_id: Principal,
    pub native_cycles: u128,
    pub available_liquid_cycles: u128,
    pub required_liquid_cycles: u128,
}

/// Protected inspection result; a reserve failure never implies that the target is absent.
pub enum CanisterInspectionOutcome {
    Status(Box<CanisterStatusResponse>),
    ReserveRequired(CanisterInspectionReserveResponse),
}

/// Replicated upstream management history for one exact inspection target.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CanisterHistoryResponse {
    pub canister_id: ic_platform::Principal,
    pub history_candid: Vec<u8>,
}

//
// CanisterInfo
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct CanisterInfo {
    pub pid: ic_platform::Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<ic_platform::Principal>,
    pub module_hash: Option<Vec<u8>>,
    pub created_at: u64,
}

//
// CanisterStatusResponse
//

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct CanisterStatusResponse {
    pub version: u64,
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

//
// CanisterSettings
//

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct CanisterSettings {
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

//
// EnvironmentVariable
//

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
}

//
// MemoryMetrics
//

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
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

//
// QueryStats
//

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
pub struct QueryStats {
    pub num_calls_total: Nat,
    pub num_instructions_total: Nat,
    pub request_payload_bytes_total: Nat,
    pub response_payload_bytes_total: Nat,
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde::de::DeserializeOwned;
    use std::fmt::Debug;

    #[test]
    fn canister_status_enums_roundtrip_wire_through_dto_path() {
        assert_enum_wire_contract(CanisterStatusType::Stopping);
        assert_enum_wire_contract(LogVisibility::AllowedViewers(vec![Principal::anonymous()]));
    }

    fn assert_enum_wire_contract<T>(value: T)
    where
        T: serde::Serialize + Clone + Debug + DeserializeOwned + Eq,
    {
        let bytes = crate::protocol::authority::encode(&value).expect("encode canister enum");
        let decoded =
            crate::protocol::authority::decode::<T>(&bytes).expect("decode canister enum");

        assert_eq!(decoded, value);
    }
}
