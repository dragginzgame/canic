//! Pure value and decision helpers used by higher-level runtime layers.
//!
//! `domain` owns deterministic computation and error composition, but it does
//! not perform storage access or orchestration.

pub mod icrc;
pub mod policy;

use crate::{InternalError, InternalErrorOrigin};
use thiserror::Error as ThisError;

///
/// DomainError
///

#[derive(Debug, ThisError)]
pub enum DomainError {
    #[error(transparent)]
    Policy(#[from] policy::PolicyError),
}

impl From<DomainError> for InternalError {
    fn from(err: DomainError) -> Self {
        Self::domain(InternalErrorOrigin::Domain, err.to_string())
    }
}
