//! Pure domain policy decisions.
//!
//! This namespace owns side-effect-free decisions only. It must not read
//! storage, call IC/runtime APIs, spawn timers, serialize wire/storage payloads,
//! or mutate state.

pub mod auth;
pub mod authority_restore;
#[cfg(feature = "blob-storage-billing")]
pub mod blob_storage;
pub mod component_allocation;
pub mod component_child_allocation;
pub mod cycles;
pub mod cycles_funding;
pub mod env;
pub mod fleet_activation;
pub mod icp_refill;
pub mod intent;
pub mod log;
pub mod placement;

use crate::InternalError;
use thiserror::Error as ThisError;

///
/// PolicyError
///

#[derive(Debug, ThisError)]
pub enum PolicyError {
    #[error(transparent)]
    AuthPolicy(#[from] auth::AuthPolicyError),

    #[error(transparent)]
    AuthorityRestorePolicy(#[from] authority_restore::AuthorityRestoreEndpointPolicyError),

    #[error(transparent)]
    EnvPolicy(#[from] env::EnvPolicyError),

    #[error(transparent)]
    FleetActivationPolicy(#[from] fleet_activation::FleetActivationEndpointPolicyError),

    #[error(transparent)]
    ScalingPolicy(#[from] placement::scaling::ScalingPolicyError),
}

impl From<PolicyError> for InternalError {
    fn from(err: PolicyError) -> Self {
        use crate::diagnostics::codes;

        let code = match err {
            PolicyError::AuthPolicy(err) => match err {
                auth::AuthPolicyError::PublicPrepareScopeNotSelfGrantable { .. }
                | auth::AuthPolicyError::RootIssuerAudienceNotAllowed { .. }
                | auth::AuthPolicyError::RootIssuerGrantNotAllowed { .. } => {
                    codes::AUTHORITY_UNAUTHORIZED
                }
                auth::AuthPolicyError::RootIssuerFleetMismatch
                | auth::AuthPolicyError::RootIssuerPolicyMismatch { .. } => {
                    codes::AUTHORITY_CONFLICT
                }
                auth::AuthPolicyError::RootIssuerAudienceRequired
                | auth::AuthPolicyError::RootIssuerGrantRequired
                | auth::AuthPolicyError::RootIssuerRenewalGrantRequired => {
                    codes::CONFIGURATION_INCOMPLETE
                }
                auth::AuthPolicyError::RootIssuerCertTtlZero
                | auth::AuthPolicyError::RootIssuerMaxCertTtlZero
                | auth::AuthPolicyError::RootIssuerRefreshAfterInvalid
                | auth::AuthPolicyError::RootIssuerRefreshRatioInvalid { .. } => {
                    codes::CONFIGURATION_INVALID
                }
                auth::AuthPolicyError::RootIssuerCertTtlExceedsMax { .. } => {
                    codes::TIME_INVALID_STATE
                }
                auth::AuthPolicyError::RootIssuerDisabled { .. } => codes::SECURITY_INACTIVE,
                auth::AuthPolicyError::RootIssuerRefreshAfterOverflow => codes::TIME_CAPACITY,
                auth::AuthPolicyError::RootIssuerUnregistered => codes::AUTHORITY_UNAVAILABLE,
            },
            PolicyError::AuthorityRestorePolicy(_) => codes::AUTHORITY_INACTIVE,
            PolicyError::EnvPolicy(_) => codes::CONFIGURATION_INCOMPLETE,
            PolicyError::FleetActivationPolicy(_) => codes::LIFECYCLE_INACTIVE,
            PolicyError::ScalingPolicy(err) => match err {
                placement::scaling::ScalingPolicyError::ScalingDisabled => {
                    codes::CONFIGURATION_INACTIVE
                }
                placement::scaling::ScalingPolicyError::PoolNotFound(_) => {
                    codes::CONFIGURATION_UNAVAILABLE
                }
            },
        };
        Self::public(code)
    }
}
