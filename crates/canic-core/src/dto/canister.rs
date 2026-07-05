use crate::dto::prelude::*;

pub use crate::domain::canister::{CanisterStatusType, LogVisibility};

//
// CanisterInfo
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CanisterInfo {
    pub pid: Principal,
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
    pub created_at: u64,
}

//
// CanisterStatusResponse
//

#[derive(CandidType, Clone, Debug, Deserialize)]
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

//
// CanisterSettings
//

#[derive(CandidType, Clone, Debug, Deserialize)]
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

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
}

//
// MemoryMetrics
//

#[derive(CandidType, Clone, Debug, Deserialize)]
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

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct QueryStats {
    pub num_calls_total: Nat,
    pub num_instructions_total: Nat,
    pub request_payload_bytes_total: Nat,
    pub response_payload_bytes_total: Nat,
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{Decode, Encode};
    use serde::de::DeserializeOwned;
    use std::fmt::Debug;

    #[test]
    fn canister_status_enums_roundtrip_candid_through_dto_path() {
        assert_enum_candid_contract(CanisterStatusType::Stopping);
        assert_enum_candid_contract(LogVisibility::AllowedViewers(vec![Principal::anonymous()]));
    }

    fn assert_enum_candid_contract<T>(value: T)
    where
        T: CandidType + Clone + Debug + DeserializeOwned + Eq,
    {
        let bytes = Encode!(&value).expect("encode canister enum");
        let decoded = Decode!(&bytes, T).expect("decode canister enum");

        assert_eq!(decoded, value);
    }
}
