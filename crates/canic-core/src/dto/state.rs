use crate::dto::prelude::*;

///
/// AppCommand
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum AppCommand {
    Start,
    Readonly,
    Stop,
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
}

///
/// AppStateResponse
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppStateResponse {
    pub mode: AppMode,
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
