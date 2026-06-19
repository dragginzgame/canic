//! Module: infra::ic::nns
//!
//! Responsibility: group raw NNS canister infra adapters.
//! Does not own: topology policy, registry storage, or endpoint response mapping.
//! Boundary: ops topology calls this for raw NNS lookups.

pub mod registry;

use crate::infra::{InfraError, ic::IcInfraError};
use thiserror::Error as ThisError;

///
/// NnsInfraError
///
/// NNS infra failure wrapper converted into IC infra errors.
/// Owned by NNS infra and returned by raw NNS adapters.
///

#[derive(Debug, ThisError)]
pub enum NnsInfraError {
    #[error(transparent)]
    NnsRegistryInfra(#[from] registry::NnsRegistryInfraError),
}

impl From<NnsInfraError> for InfraError {
    fn from(err: NnsInfraError) -> Self {
        IcInfraError::from(err).into()
    }
}
