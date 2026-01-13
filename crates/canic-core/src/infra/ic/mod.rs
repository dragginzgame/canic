//! IC-related infra helpers.
//!
//! This module groups low-level IC concerns (management canister calls, ICC call
//! wrappers, HTTP outcalls, timers) under a single namespace to keep `infra/`
//! navigable.

pub mod call;
pub mod http;
pub mod ledger;
pub mod mgmt;
pub mod network;
pub mod nns;
pub mod signature;

use crate::cdk::{
    call::{CallFailed, CandidDecodeFailed, Error as CallError},
    candid::Error as CandidError,
};
use thiserror::Error as ThisError;

///
/// IcInfraError
///

#[derive(Debug, ThisError)]
pub enum IcInfraError {
    #[error(transparent)]
    HttpInfra(#[from] http::HttpInfraError),

    #[error(transparent)]
    LedgerInfra(#[from] ledger::LedgerInfraError),

    #[error(transparent)]
    MgmtInfra(#[from] mgmt::MgmtInfraError),

    #[error(transparent)]
    NnsInfra(#[from] nns::NnsInfraError),

    #[error(transparent)]
    SignatureInfra(#[from] signature::SignatureInfraError),

    // candid catch-all errors
    #[error(transparent)]
    CallFailed(#[from] CallFailed),

    #[error(transparent)]
    Candid(#[from] CandidError),

    #[error(transparent)]
    CandidDecode(#[from] CandidDecodeFailed),
}

impl From<CallError> for IcInfraError {
    fn from(err: CallError) -> Self {
        match err {
            CallError::CandidDecodeFailed(err) => err.into(),
            CallError::InsufficientLiquidCycleBalance(err) => {
                CallFailed::InsufficientLiquidCycleBalance(err).into()
            }
            CallError::CallPerformFailed(err) => CallFailed::CallPerformFailed(err).into(),
            CallError::CallRejected(err) => CallFailed::CallRejected(err).into(),
        }
    }
}
