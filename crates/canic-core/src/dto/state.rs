use crate::dto::prelude::*;

pub use crate::model::memory::state::{AppStateView, SubnetStateView};

///
/// AppCommand
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Display, Eq, PartialEq)]
pub enum AppCommand {
    Start,
    Readonly,
    Stop,
}
