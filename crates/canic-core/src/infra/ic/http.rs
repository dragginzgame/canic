//! Module: infra::ic::http
//!
//! Responsibility: perform raw IC HTTP outcalls.
//! Does not own: HTTP policy, response validation, metrics, or workflow retries.
//! Boundary: ops calls this adapter with already-approved HTTP request arguments.

use crate::{
    cdk::mgmt::{HttpRequestArgs, HttpRequestResult},
    infra::{InfraError, ic::IcInfraError},
};
use thiserror::Error as ThisError;

///
/// HttpInfraError
///
/// Raw HTTP outcall failure surfaced by IC infra.
/// Owned by HTTP infra and converted into `InfraError`.
///

#[derive(Debug, ThisError)]
pub enum HttpInfraError {
    #[error(transparent)]
    RequestFailed(#[from] crate::cdk::call::Error),
}

impl From<HttpInfraError> for InfraError {
    fn from(err: HttpInfraError) -> Self {
        IcInfraError::from(err).into()
    }
}

///
/// HttpInfra
///
/// Raw IC HTTP outcall facade.
/// Owned by IC infra and used by ops HTTP adapters.
///

pub struct HttpInfra;

impl HttpInfra {
    /// Raw IC HTTP request passthrough.
    ///
    /// No metrics, no validation, no interpretation.
    pub async fn http_request_raw(args: &HttpRequestArgs) -> Result<HttpRequestResult, InfraError> {
        let result = crate::cdk::mgmt::http_request(args)
            .await
            .map_err(HttpInfraError::from)?;

        Ok(result)
    }
}
