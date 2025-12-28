use crate::dto::prelude::*;

///
/// AppCommand
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Display, Eq, PartialEq)]
pub enum AppCommand {
    Start,
    Readonly,
    Stop,
}

///
/// AppModeView
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AppModeView {
    Enabled,
    Readonly,
    Disabled,
}

///
/// AppStateView
/// Read-only snapshot of application state for transfer and inspection.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppStateView {
    pub mode: AppModeView,
}

///
/// SubnetStateView
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetStateView {}
