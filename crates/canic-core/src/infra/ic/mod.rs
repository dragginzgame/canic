//! Module: infra::ic
//!
//! Responsibility: group raw IC call, NNS, and management adapters.
//! Does not own: ops conversion, workflow policy, or endpoint DTO shaping.
//! Boundary: ops calls this namespace for low-level IC platform interactions.

pub mod build_network;
pub mod call;
pub mod icp_refill;
pub mod known;
pub mod mgmt;
pub mod nns;

use crate::cdk::{
    call::{CallFailed, CandidDecodeFailed, Error as CallError},
    candid::Error as CandidError,
};
use thiserror::Error as ThisError;

///
/// IcInfraError
///
/// IC infra failure wrapper for raw IC transport and protocol adapters.
/// Owned by `infra::ic` and converted into `InfraError`.
///

#[derive(Debug, ThisError)]
pub enum IcInfraError {
    #[error(transparent)]
    IcpRefillInfra(#[from] icp_refill::IcpRefillInfraError),

    #[error(transparent)]
    MgmtInfra(#[from] mgmt::MgmtInfraError),

    #[error(transparent)]
    NnsInfra(#[from] nns::NnsInfraError),

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
