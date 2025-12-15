//! IC-related ops helpers.
//!
//! This module groups low-level IC concerns (management canister calls, ICC call
//! wrappers, HTTP outcalls, timers) under a single namespace to keep `ops/`
//! navigable.

pub mod call;
pub mod http;
pub mod icrc;
pub mod mgmt;
pub mod provision;
pub mod signature;
pub mod timer;

pub use mgmt::*;

use crate::{Error, ThisError, ops::OpsError};

///
/// IcOpsError
///

#[derive(Debug, ThisError)]
pub enum IcOpsError {
    #[error(transparent)]
    ProvisionOpsError(#[from] provision::ProvisionOpsError),

    #[error(transparent)]
    SignatureOpsError(#[from] signature::SignatureOpsError),
}

impl From<IcOpsError> for Error {
    fn from(err: IcOpsError) -> Self {
        OpsError::from(err).into()
    }
}
