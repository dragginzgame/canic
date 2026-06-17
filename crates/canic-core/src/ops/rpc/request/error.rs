//! Module: ops::rpc::request::error
//!
//! Responsibility: define request dispatch errors for RPC ops.
//! Does not own: workflow error mapping or public endpoint DTOs.
//! Boundary: converts request dispatch failures into the shared RPC ops error path.

use crate::{InternalError, infra::InfraError, ops::rpc::RpcOpsError};
use thiserror::Error as ThisError;

///
/// RequestOpsError
///
/// Errors produced during request dispatch or response handling.
///

#[derive(Debug, ThisError)]
pub enum RequestOpsError {
    #[error(transparent)]
    Infra(#[from] InfraError),

    #[error("invalid response type")]
    InvalidResponseType,
}

impl From<RequestOpsError> for InternalError {
    fn from(err: RequestOpsError) -> Self {
        RpcOpsError::from(err).into()
    }
}
