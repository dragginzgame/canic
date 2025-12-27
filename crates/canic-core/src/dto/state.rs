use crate::{dto::prelude::*, model::memory::state::AppMode};

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
/// AppStateView
/// Read-only snapshot of application state for transfer and inspection.
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppStateView {
    pub mode: AppMode,
}

///
/// SubnetStateView
///

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetStateView {}
