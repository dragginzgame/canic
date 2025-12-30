pub use crate::cdk::mgmt::{HttpHeader, HttpMethod, HttpRequestArgs, HttpRequestResult};

///
/// Raw IC HTTP request passthrough.
/// No metrics, no validation, no interpretation.
///

pub async fn http_request_raw(
    args: &HttpRequestArgs,
) -> Result<HttpRequestResult, crate::cdk::call::Error> {
    crate::cdk::mgmt::http_request(args).await
}
