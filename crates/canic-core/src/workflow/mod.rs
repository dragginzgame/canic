pub mod app;
pub mod bootstrap;
pub mod cascade;
pub mod command;
pub mod directory;
pub mod ic;
pub mod orchestrator;
pub mod placement;
pub mod pool;
pub mod random;
pub mod rpc;
pub mod runtime;
pub mod timer;

use crate::{
    ThisError,
    ops::{env::EnvData, storage::directory::DirectoryView},
};
use candid::CandidType;
use serde::Deserialize;

///
/// WorkflowError
///

#[derive(Debug, ThisError)]
pub enum WorkflowError {
    #[error(transparent)]
    CascadeError(#[from] cascade::CascadeError),

    #[error(transparent)]
    IcError(#[from] ic::IcError),

    #[error(transparent)]
    OrchestrationError(#[from] orchestrator::OrchestratorError),
}

///
/// CanisterInitPayload
/// todo - move this somewhere
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
