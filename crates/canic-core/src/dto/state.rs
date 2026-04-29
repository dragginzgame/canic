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
// SubnetRootPublicKeyInput
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SubnetRootPublicKeyInput {
    pub public_key_sec1: Vec<u8>,
    pub key_name: String,
    pub key_hash: [u8; 32],
}

//
// SubnetAuthStateInput
//

#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct SubnetAuthStateInput {
    pub delegated_root_public_key: Option<SubnetRootPublicKeyInput>,
}

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
