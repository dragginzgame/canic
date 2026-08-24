//! Module: ops::storage
//!
//! Responsibility: group deterministic storage operations and shared errors.
//! Does not own: stable record schemas, workflow orchestration, or endpoint DTOs.
//! Boundary: ops layer between workflows and stable storage facades.

pub mod async_job_recovery;
pub mod auth;
pub mod authority_restore;
mod canister;
pub mod children;
pub mod cycles;
pub mod fleet_activation;
pub mod fleet_admission_projection;
pub mod icp_refill;
pub mod intent;
pub mod placement;
pub mod replay;
pub mod state;

use crate::InternalError;
use thiserror::Error as ThisError;

///
/// StorageOpsError
///
/// Typed failure surface shared across storage operation submodules.
///

#[derive(Debug, ThisError)]
pub enum StorageOpsError {
    #[error(transparent)]
    FleetActivationOps(#[from] fleet_activation::FleetActivationOpsError),

    #[error(transparent)]
    IntentStoreOps(#[from] intent::IntentStoreOpsError),

    #[error(transparent)]
    IcpRefillRecordOps(#[from] icp_refill::IcpRefillRecordOpsError),

    #[error(transparent)]
    PlacementIndexRegistryOps(#[from] placement::index::PlacementIndexRegistryOpsError),

    #[cfg(feature = "sharding")]
    #[error(transparent)]
    ShardingRegistryOps(#[from] placement::sharding::ShardingRegistryOpsError),
}

impl From<StorageOpsError> for InternalError {
    fn from(err: StorageOpsError) -> Self {
        use crate::diagnostics::codes;

        match err {
            StorageOpsError::FleetActivationOps(err) => {
                let code = match err {
                    fleet_activation::FleetActivationOpsError::Admission(_) => {
                        codes::CONFIGURATION_INVALID
                    }
                    fleet_activation::FleetActivationOpsError::Encode(_) => codes::CODEC_FAILED,
                    fleet_activation::FleetActivationOpsError::RecordTooLarge { .. } => {
                        codes::CAPACITY_LIMIT
                    }
                    fleet_activation::FleetActivationOpsError::AlreadyInitialized => {
                        codes::STATE_INVALID_STATE
                    }
                    fleet_activation::FleetActivationOpsError::NotInitialized => {
                        codes::STATE_UNAVAILABLE
                    }
                    fleet_activation::FleetActivationOpsError::InvalidRecord { .. } => {
                        codes::STATE_INVALID
                    }
                    fleet_activation::FleetActivationOpsError::NotActive => codes::STATE_INACTIVE,
                    fleet_activation::FleetActivationOpsError::IdentityMismatch => {
                        codes::AUTHORITY_CONFLICT
                    }
                    fleet_activation::FleetActivationOpsError::EvidenceMismatch => {
                        codes::EVIDENCE_CONFLICT
                    }
                    fleet_activation::FleetActivationOpsError::InvalidTransition { .. } => {
                        codes::LIFECYCLE_INVALID_STATE
                    }
                };
                Self::public(code)
            }
            StorageOpsError::IntentStoreOps(err) => err.into(),
            StorageOpsError::IcpRefillRecordOps(err) => err.into(),
            StorageOpsError::PlacementIndexRegistryOps(err) => err.into(),
            #[cfg(feature = "sharding")]
            StorageOpsError::ShardingRegistryOps(err) => err.into(),
        }
    }
}
