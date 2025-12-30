use crate::{
    dto::directory::{AppDirectoryView, SubnetDirectoryView},
    dto::env::EnvView,
};
use candid::CandidType;
use serde::Deserialize;

///
/// CanisterInitPayload
///

#[derive(CandidType, Debug, Deserialize)]
pub struct CanisterInitPayload {
    pub env: EnvView,
    pub app_directory: AppDirectoryView,
    pub subnet_directory: SubnetDirectoryView,
}

impl CanisterInitPayload {
    #[must_use]
    pub const fn new(
        env: EnvView,
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
