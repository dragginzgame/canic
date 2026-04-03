use crate::dto::prelude::*;

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

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct SubnetStateInput;

///
/// SubnetStateResponse
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct SubnetStateResponse;

///
/// BootstrapStatusResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct BootstrapStatusResponse {
    pub ready: bool,
    pub phase: String,
    pub last_error: Option<String>,
}
