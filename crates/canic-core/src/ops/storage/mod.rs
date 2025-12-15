pub mod directory;
pub mod env;
pub mod registry;
pub mod sharding;
pub mod state;
pub mod topology;

pub use crate::model::memory::{CanisterEntry, CanisterSummary};

use crate::{
    Error, ThisError,
    ops::{
        OpsError,
        storage::{
            directory::{AppDirectoryOpsError, DirectoryView, SubnetDirectoryOpsError},
            env::{EnvData, EnvOpsError},
            registry::MemoryRegistryOpsError,
            state::AppStateOpsError,
            topology::TopologyOpsError,
        },
    },
};
use candid::CandidType;
use serde::Deserialize;

///
/// StorageOpsError
///

#[derive(Debug, ThisError)]
pub enum StorageOpsError {
    #[error(transparent)]
    AppDirectoryOpsError(#[from] AppDirectoryOpsError),

    #[error(transparent)]
    AppStateOpsError(#[from] AppStateOpsError),

    #[error(transparent)]
    EnvOpsError(#[from] EnvOpsError),

    #[error(transparent)]
    MemoryRegistryOpsError(#[from] MemoryRegistryOpsError),

    #[error(transparent)]
    SubnetDirectoryOpsError(#[from] SubnetDirectoryOpsError),

    #[error(transparent)]
    TopologyOpsError(#[from] TopologyOpsError),
}

impl From<StorageOpsError> for Error {
    fn from(err: StorageOpsError) -> Self {
        OpsError::StorageOpsError(err).into()
    }
}

///
/// CanisterInitPayload
///

#[derive(CandidType, Debug, Default, Deserialize)]
pub struct CanisterInitPayload {
    pub env: EnvData,
    pub app_directory: DirectoryView,
    pub subnet_directory: DirectoryView,
}

impl CanisterInitPayload {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}
