use crate::dto::{
    env::EnvBootstrapArgs,
    prelude::*,
    topology::{AppDirectoryArgs, SubnetDirectoryArgs},
};

///
/// CanisterInitPayload
///

#[derive(CandidType, Debug, Deserialize)]
pub struct CanisterInitPayload {
    pub env: EnvBootstrapArgs,
    pub app_directory: AppDirectoryArgs,
    pub subnet_directory: SubnetDirectoryArgs,
}
