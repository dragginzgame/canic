use crate::{ThisError, infra::InfraError, infra::ic::IcInfraError};

///
/// CDK Http Imports
///
pub use crate::cdk::mgmt::{HttpHeader, HttpMethod, HttpRequestArgs, HttpRequestResult};

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
/// Raw IC HTTP request passthrough.
/// No metrics, no validation, no interpretation.
///

pub async fn http_request_raw(args: &HttpRequestArgs) -> Result<HttpRequestResult, InfraError> {
    let result = crate::cdk::mgmt::http_request(args)
        .await
        .map_err(HttpInfraError::from)?;

    Ok(result)
}
