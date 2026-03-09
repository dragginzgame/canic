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

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AppStatus {
    Active,
    Readonly,
    Stopped,
}

///
/// AppMode
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AppMode {
    Enabled,
    Readonly,
    Disabled,
}

///
/// AppStateInput
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppStateInput {
    pub mode: AppMode,
    pub cycles_funding_enabled: bool,
}

///
/// AppStateResponse
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppStateResponse {
    pub mode: AppMode,
    pub cycles_funding_enabled: bool,
}

///
/// SubnetStateInput
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetStateInput {}

///
/// SubnetStateResponse
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetStateResponse {}
