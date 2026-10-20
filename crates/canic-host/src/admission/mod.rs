//! Fleet admission errors and explicit target context.
//!
//! Host owns authority and operations; command parsing and rendering belong to CLI.

pub mod model;
mod ops;
pub mod view;
pub mod workflow;

use crate::{
    CanisterProtocolError, fleet_ensure::CurrentFleetInventoryError, icp_config::IcpConfigError,
    protocol_binding::ReleaseProtocolBindingError,
};
use thiserror::Error as ThisError;

/// Failures from the protected Fleet-admission operator surface.
#[derive(Debug, ThisError)]
pub enum AdmissionError {
    #[error("failed to read Canic Fleet state: {0}")]
    IcpRoot(#[source] IcpConfigError),

    #[error(transparent)]
    CurrentFleet(#[from] CurrentFleetInventoryError),

    #[error(transparent)]
    Protocol(#[from] CanisterProtocolError),

    #[error(transparent)]
    ProtocolBinding(#[from] ReleaseProtocolBindingError),

    #[error("invalid admission selector: {0}")]
    InvalidSelector(String),

    #[error("invalid Fleet-admission mutation: {0}")]
    InvalidMutation(String),

    #[error("current Fleet-admission authority is invalid: {0}")]
    Authority(String),
}

/// Explicit executable and environment selected by an operator or library caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissionTarget {
    pub environment: String,
    pub icp: String,
}

const ADMISSION_PLAN_SCHEMA_VERSION: u16 = 1;
const ROOT_STATUS_PAGE_SIZE: u64 = 32;

fn authority_error(message: impl Into<String>) -> AdmissionError {
    AdmissionError::Authority(message.into())
}
