use crate::ids::WasmStoreBinding;
use candid::{CandidType, Principal};
use serde::Deserialize;

///
/// WasmStoreStateResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreStateResponse {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
    pub created_at: u64,
}

///
/// SubnetStateResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SubnetStateResponse {
    pub wasm_stores: Vec<WasmStoreStateResponse>,
}
