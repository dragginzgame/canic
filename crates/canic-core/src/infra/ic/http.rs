use crate::{
    cdk::mgmt::{HttpRequestArgs, HttpRequestResult},
    infra::{InfraError, ic::IcInfraError},
};
use thiserror::Error as ThisError;

///
/// HttpInfraError
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

pub struct HttpInfra;

impl HttpInfra {
    /// Raw IC HTTP request passthrough.
    /// No metrics, no validation, no interpretation.
    pub async fn http_request_raw(args: &HttpRequestArgs) -> Result<HttpRequestResult, InfraError> {
        let result = crate::cdk::mgmt::http_request(args)
            .await
            .map_err(HttpInfraError::from)?;

        Ok(result)
    }
}
