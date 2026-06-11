use crate::dto::prelude::*;

//
// AppCommand
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum AppCommand {
    SetStatus(AppStatus),
    SetCyclesFundingEnabled(bool),
}

//
// AppCommandResponse
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum AppCommandResponse {
    Status(SetStateResponse<AppStatus>),
    CyclesFundingEnabled(SetStateResponse<bool>),
}

//
// SetStateResponse
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct SetStateResponse<T> {
    pub previous: T,
    pub current: T,
    pub changed: bool,
}

//
// AppStatus
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum AppStatus {
    Active,
    Readonly,
    Stopped,
}

//
// AppMode
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum AppMode {
    Enabled,
    Readonly,
    Disabled,
}

//
// AppStateInput
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct AppStateInput {
    pub mode: AppMode,
    pub cycles_funding_enabled: bool,
}

//
// AppStateResponse
//

#[derive(CandidType, Deserialize)]
pub struct AppStateResponse {
    pub mode: AppMode,
    pub cycles_funding_enabled: bool,
}

//
// SubnetAuthStateInput
//

#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct SubnetAuthStateInput {}

//
// SubnetStateInput
//

#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct SubnetStateInput {
    pub auth: SubnetAuthStateInput,
}

//
// SubnetStateResponse
//

#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct SubnetStateResponse {
    pub auth: SubnetAuthStateInput,
}

//
// BootstrapStatusResponse
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct BootstrapStatusResponse {
    pub ready: bool,
    pub phase: String,
    pub last_error: Option<String>,
}
