//! Module: infra
//!
//! Responsibility: expose low-level platform adapters and infra-scoped failures.
//! Does not own: workflow orchestration, policy decisions, or storage mutation.
//! Boundary: ops calls infra for mechanical platform effects and raw transport.

pub mod ic;

use crate::{InternalError, InternalErrorOrigin};

impl From<ic::IcInfraError> for InternalError {
    fn from(err: ic::IcInfraError) -> Self {
        Self::infra(InternalErrorOrigin::Infra, err.to_string())
    }
}
