use crate::{
    dto::directory::{AppDirectoryView, SubnetDirectoryView},
    ops::env::EnvData,
};
use candid::CandidType;
use serde::Deserialize;

///
/// CanisterInitPayload
///

#[derive(CandidType, Debug, Deserialize)]
pub struct CanisterInitPayload {
    pub env: EnvData,
    pub app_directory: AppDirectoryView,
    pub subnet_directory: SubnetDirectoryView,
}

impl CanisterInitPayload {
    #[must_use]
    pub const fn new(
        env: EnvData,
        app_directory: AppDirectoryView,
        subnet_directory: SubnetDirectoryView,
    ) -> Self {
        Self {
            env,
            app_directory,
            subnet_directory,
        }
    }
}
