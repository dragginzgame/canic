//! Module: infra
//!
//! Responsibility: expose low-level platform adapters and infra-scoped failures.
//! Does not own: workflow orchestration, policy decisions, or storage mutation.
//! Boundary: ops calls infra for mechanical platform effects and raw transport.

pub mod ic;

use crate::{InternalError, InternalErrorOrigin};
use thiserror::Error as ThisError;

///
/// InfraError
///
/// Infra-layer failure wrapper converted to the workspace internal error shape.
/// Owned by infra and returned to ops callers crossing platform boundaries.
///

#[derive(Debug, ThisError)]
pub enum InfraError {
    #[error(transparent)]
    IcInfra(#[from] ic::IcInfraError),
}

impl From<InfraError> for InternalError {
    fn from(err: InfraError) -> Self {
        Self::infra(InternalErrorOrigin::Infra, err.to_string())
    }
}
