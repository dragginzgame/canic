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
