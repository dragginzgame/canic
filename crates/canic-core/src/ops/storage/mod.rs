pub mod cycles;
pub mod directory;
pub mod env;
pub mod memory;
pub mod pool;
pub mod scaling;
pub mod sharding;
pub mod state;
pub mod topology;

pub use crate::model::memory::{CanisterEntry, CanisterSummary};

use crate::ops::storage::env::EnvOpsError;
use crate::{
    Error, ThisError,
    ops::{
        OpsError,
        storage::{
            directory::{AppDirectoryOpsError, DirectoryView, SubnetDirectoryOpsError},
            env::EnvData,
            memory::MemoryRegistryOpsError,
            sharding::ShardingRegistryOpsError,
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
    ShardingRegistryOpsError(#[from] ShardingRegistryOpsError),

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
