use crate::{
    interface::prelude::*, model::memory::env::EnvData,
    ops::model::memory::directory::DirectoryView,
};
use serde::Deserialize;

///
/// CanisterInitPayload
///

#[derive(CandidType, Debug, Default, Deserialize)]
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
