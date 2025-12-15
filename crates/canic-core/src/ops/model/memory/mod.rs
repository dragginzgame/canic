pub mod cycles;
pub mod directory;
pub mod env;
pub mod log;
pub mod registry;
pub mod reserve;
pub mod scaling;
pub mod sharding;
pub mod state;
pub mod topology;

pub use crate::model::memory::{CanisterEntry, CanisterSummary};

use crate::{
    Error, ThisError,
    ops::model::{
        ModelOpsError,
        memory::{
            directory::{AppDirectoryOpsError, DirectoryView, SubnetDirectoryOpsError},
            env::{EnvData, EnvOps, EnvOpsError},
            registry::MemoryRegistryOpsError,
            scaling::ScalingOpsError,
            sharding::ShardingOpsError,
            state::AppStateOpsError,
            topology::TopologyOpsError,
        },
    },
};
use candid::CandidType;
use serde::Deserialize;

///
/// MemoryOpsError
///

#[derive(Debug, ThisError)]
pub enum MemoryOpsError {
    #[error(transparent)]
    AppDirectoryOpsError(#[from] AppDirectoryOpsError),

    #[error(transparent)]
    AppStateOpsError(#[from] AppStateOpsError),

    #[error(transparent)]
    EnvOpsError(#[from] EnvOpsError),

    #[error(transparent)]
    MemoryRegistryOpsError(#[from] MemoryRegistryOpsError),

    #[error(transparent)]
    ScalingOpsError(#[from] ScalingOpsError),

    #[error(transparent)]
    ShardingOpsError(#[from] ShardingOpsError),

    #[error(transparent)]
    SubnetDirectoryOpsError(#[from] SubnetDirectoryOpsError),

    #[error(transparent)]
    TopologyOpsError(#[from] TopologyOpsError),
}

impl From<MemoryOpsError> for Error {
    fn from(err: MemoryOpsError) -> Self {
        ModelOpsError::MemoryOpsError(err).into()
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
