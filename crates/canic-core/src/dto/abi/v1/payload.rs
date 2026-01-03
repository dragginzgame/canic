use crate::dto::{
    env::EnvView,
    prelude::*,
    topology::{AppDirectoryView, SubnetDirectoryView},
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
