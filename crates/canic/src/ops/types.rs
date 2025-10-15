use crate::{
    interface::prelude::*,
    memory::{directory::DirectoryView, env::EnvData},
};
use serde::Deserialize;

///
/// CanisterInitPayload
///

#[derive(Debug, CandidType, Deserialize)]
pub struct CanisterInitPayload {
    pub env: EnvData,
    pub app_directory: DirectoryView,
    pub subnet_directory: DirectoryView,
}
