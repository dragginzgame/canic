pub mod assign;
pub mod hrw;
pub mod manage;
pub mod policy;

pub use {assign::*, manage::*, policy::*};

use crate::{
    Error, ThisError,
    ops::{OpsError, ext::ExtensionError},
    types::Principal,
};

///
/// ShardingError
/// Logical or configuration errors that occur during sharding planning.
///

#[derive(Debug, ThisError)]
pub enum ShardingError {
    #[error("shard pool not found: {0}")]
    PoolNotFound(String),

    #[error("shard cap reached")]
    ShardCapReached,

    #[error("shard creation blocked: {0}")]
    ShardCreationBlocked(String),

    #[error("sharding disabled")]
    ShardingDisabled,

    #[error("tenant '{0}' not found")]
    TenantNotFound(Principal),
}

impl From<ShardingError> for Error {
    fn from(err: ShardingError) -> Self {
        OpsError::ExtensionError(ExtensionError::ShardingError(err)).into()
    }
}
