use crate::dto::{
    directory::{AppDirectoryView, SubnetDirectoryView},
    env::EnvView,
    prelude::*,
};

///
/// CanisterInitPayload
///

#[derive(CandidType, Debug, Deserialize)]
pub struct CanisterInitPayload {
    pub env: EnvView,
    pub app_directory: AppDirectoryView,
    pub subnet_directory: SubnetDirectoryView,
}
