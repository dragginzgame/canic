//! Pure value and decision helpers used by higher-level runtime layers.
//!
//! `domain` owns deterministic computation and error composition, but it does
//! not perform storage access or orchestration.

pub mod auth;
pub mod blob_storage;
pub mod canister;
pub mod cycles;
pub mod icp_refill;
pub mod icrc;
pub mod memory;
pub mod metrics;
pub mod policy;
pub mod pool;
pub mod runtime;
pub mod state;
pub mod subnet;
pub mod value;

use crate::{InternalError, InternalErrorOrigin};
use thiserror::Error as ThisError;

///
/// DomainError
///

#[derive(Debug, ThisError)]
pub enum DomainError {
    #[error(transparent)]
    Policy(#[from] policy::pure::PolicyError),
}

impl From<DomainError> for InternalError {
    fn from(err: DomainError) -> Self {
        Self::domain(InternalErrorOrigin::Domain, err.to_string())
    }
}
