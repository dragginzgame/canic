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

use crate::{
    cdk::{
        call::{CallFailed, CandidDecodeFailed},
        candid::Error as CandidError,
    },
    infra::prelude::*,
};

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
