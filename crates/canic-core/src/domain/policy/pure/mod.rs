//! Pure domain policy decisions.
//!
//! This namespace owns side-effect-free decisions only. It must not read
//! storage, call IC/runtime APIs, spawn timers, serialize wire/storage payloads,
//! or mutate state.

pub mod auth;
#[cfg(feature = "blob-storage-billing")]
pub mod blob_storage;
pub mod cycles;
pub mod cycles_funding;
pub mod env;
pub mod fleet_activation;
pub mod icp_refill;
pub mod intent;
pub mod log;
pub mod placement;
pub mod pool;
pub mod topology;
pub mod upgrade;

use crate::{InternalError, InternalErrorOrigin};
use thiserror::Error as ThisError;

///
/// PolicyError
///

#[derive(Debug, ThisError)]
pub enum PolicyError {
    #[error(transparent)]
    AuthPolicy(#[from] auth::AuthPolicyError),

    #[error(transparent)]
    EnvPolicy(#[from] env::EnvPolicyError),

    #[error(transparent)]
    FleetActivationPolicy(#[from] fleet_activation::FleetActivationEndpointPolicyError),

    #[error(transparent)]
    PoolPolicy(#[from] pool::PoolPolicyError),

    #[error(transparent)]
    TopologyPolicy(#[from] topology::TopologyPolicyError),

    #[error(transparent)]
    ScalingPolicy(#[from] placement::scaling::ScalingPolicyError),
}

impl From<PolicyError> for InternalError {
    fn from(err: PolicyError) -> Self {
        Self::domain(InternalErrorOrigin::Domain, err.to_string())
    }
}
