use crate::{
    interface::prelude::*,
    memory::{directory::DirectoryView, env::EnvData},
};
use serde::Deserialize;

///
/// CanisterInitPayload
///

#[derive(Debug, CandidType, Default, Deserialize)]
pub struct CanisterInitPayload {
    pub env: EnvData,
    pub app_directory: DirectoryView,
    pub subnet_directory: DirectoryView,
}

impl CanisterInitPayload {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}
