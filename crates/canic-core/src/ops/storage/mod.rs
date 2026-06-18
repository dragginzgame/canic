//! Module: ops::storage
//!
//! Responsibility: group deterministic storage operations and shared errors.
//! Does not own: stable record schemas, workflow orchestration, or endpoint DTOs.
//! Boundary: ops layer between workflows and stable storage facades.

pub mod auth;
pub mod children;
pub mod cycles;
pub mod icp_refill;
pub mod index;
pub mod intent;
pub mod placement;
pub mod pool;
pub mod registry;
pub mod replay;
pub mod state;

use crate::{InternalError, ops::OpsError};
use thiserror::Error as ThisError;

///
/// StorageOpsError
/// InternalError envelope shared across operations submodules
///

#[derive(Debug, ThisError)]
pub enum StorageOpsError {
    #[error(transparent)]
    IndexOps(#[from] index::IndexOpsError),

    #[error(transparent)]
    IntentStoreOps(#[from] intent::IntentStoreOpsError),

    #[error(transparent)]
    IcpRefillRecordOps(#[from] icp_refill::IcpRefillRecordOpsError),

    #[error(transparent)]
    DirectoryRegistryOps(#[from] placement::directory::DirectoryRegistryOpsError),

    #[cfg(feature = "sharding")]
    #[error(transparent)]
    ShardingRegistryOps(#[from] placement::sharding::ShardingRegistryOpsError),

    #[error(transparent)]
    SubnetRegistryOps(#[from] registry::subnet::SubnetRegistryOpsError),
}

impl From<StorageOpsError> for InternalError {
    fn from(err: StorageOpsError) -> Self {
        OpsError::StorageOps(err).into()
    }
}
