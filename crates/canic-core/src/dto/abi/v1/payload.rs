use crate::dto::{
    env::EnvBootstrapArgs,
    prelude::*,
    topology::{AppIndexArgs, SubnetIndexArgs},
};

//
// CanisterInitPayload
//

#[derive(CandidType, Debug, Deserialize)]
pub struct CanisterInitPayload {
    pub env: EnvBootstrapArgs,
    pub app_index: AppIndexArgs,
    pub subnet_index: SubnetIndexArgs,
}
