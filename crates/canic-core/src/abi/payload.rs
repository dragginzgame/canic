use crate::ops::{env::EnvData, storage::directory::DirectoryView};
use candid::CandidType;
use serde::Deserialize;

///
/// CanisterInitPayload
///

#[derive(CandidType, Debug, Deserialize)]
pub struct CanisterInitPayload {
    pub env: EnvData,
    pub app_directory: DirectoryView,
    pub subnet_directory: DirectoryView,
}

impl CanisterInitPayload {
    #[must_use]
    pub const fn new(
        env: EnvData,
        app_directory: DirectoryView,
        subnet_directory: DirectoryView,
    ) -> Self {
        Self {
            env,
            app_directory,
            subnet_directory,
        }
    }
}
