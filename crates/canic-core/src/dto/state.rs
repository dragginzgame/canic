use crate::dto::prelude::*;
use crate::ids::WasmStoreBinding;

///
/// AppCommand
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum AppCommand {
    SetStatus(AppStatus),
    SetCyclesFundingEnabled(bool),
}

///
/// AppStatus
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum AppStatus {
    Active,
    Readonly,
    Stopped,
}

///
/// AppMode
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum AppMode {
    Enabled,
    Readonly,
    Disabled,
}

///
/// AppStateInput
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct AppStateInput {
    pub mode: AppMode,
    pub cycles_funding_enabled: bool,
}

///
/// AppStateResponse
///

#[derive(CandidType, Deserialize)]
pub struct AppStateResponse {
    pub mode: AppMode,
    pub cycles_funding_enabled: bool,
}

///
/// SubnetStateInput
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreStateInput {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
    pub created_at: u64,
}

///
/// SubnetStateInput
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SubnetStateInput {
    pub wasm_stores: Option<Vec<WasmStoreStateInput>>,
}

///
/// SubnetStateResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WasmStoreStateResponse {
    pub binding: WasmStoreBinding,
    pub pid: Principal,
    pub created_at: u64,
}

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SubnetStateResponse {
    pub wasm_stores: Vec<WasmStoreStateResponse>,
}
