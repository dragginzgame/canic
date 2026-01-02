use crate::{
    Error, ThisError,
    infra::ic::http::{
        HttpHeader, HttpMethod, HttpRequestArgs, HttpRequestResult, http_request_raw,
    },
    ops::{
        prelude::*,
        runtime::metrics::{record_http_outcall, record_http_request},
    },
};
use num_traits::ToPrimitive;
use serde::de::DeserializeOwned;

/// Maximum allowed response size for HTTP outcalls.
pub const MAX_RESPONSE_BYTES: u64 = 200_000;

///
/// HttpOpsError
///

#[derive(Debug, ThisError)]
pub enum HttpOpsError {
    #[error("http error status: {0}")]
    HttpStatus(u32),

    #[error("http decode failed: {0}")]
    HttpDecode(String),
}

impl From<HttpOpsError> for Error {
    fn from(err: HttpOpsError) -> Self {
        OpsError::from(err).into()
    }
}

///
/// Http
/// Approved, observable HTTP helpers over the IC management API.
///

//
// High-level typed helpers
//

/// Perform an HTTP GET request and deserialize the JSON response.
pub async fn get<T: DeserializeOwned>(
    url: &str,
    headers: impl AsRef<[(&str, &str)]>,
) -> Result<T, Error> {
    get_with_label(url, headers, None).await
}

/// Same as `get`, with an optional metrics label.
pub async fn get_with_label<T: DeserializeOwned>(
    url: &str,
    headers: impl AsRef<[(&str, &str)]>,
    label: Option<&str>,
) -> Result<T, Error> {
    // Emit observability signals
    record_metrics(HttpMethod::GET, url, label);

    // Convert header pairs into IC HTTP headers
    let headers: Vec<HttpHeader> = headers
        .as_ref()
        .iter()
        .map(|(name, value)| HttpHeader {
            name: name.to_string(),
            value: value.to_string(),
        })
        .collect();

    // Build raw IC HTTP request arguments
    let args = HttpRequestArgs {
        url: url.to_string(),
        method: HttpMethod::GET,
        headers,
        max_response_bytes: Some(MAX_RESPONSE_BYTES),
        ..Default::default()
    };

    // Perform raw HTTP outcall via infra
    let res = http_request_raw(&args).await?;

    // Validate HTTP status code
    let status: u32 = res.status.0.to_u32().unwrap_or(0);
    if !(200..300).contains(&status) {
        return Err(HttpOpsError::HttpStatus(status).into());
    }

    // Deserialize response body
    serde_json::from_slice(&res.body)
        .map_err(|err| HttpOpsError::HttpDecode(err.to_string()).into())
}

//
// Low-level escape hatches
//

/// Perform a raw HTTP request with metrics, returning the IC response verbatim.
pub async fn get_raw(args: HttpRequestArgs) -> Result<HttpRequestResult, Error> {
    get_raw_with_label(args, None).await
}

/// Same as `get_raw`, with an optional metrics label.
pub async fn get_raw_with_label(
    args: HttpRequestArgs,
    label: Option<&str>,
) -> Result<HttpRequestResult, Error> {
    // Emit observability signals
    record_metrics(args.method, &args.url, label);

    // Delegate to infra without additional interpretation
    let res = http_request_raw(&args).await?;

    Ok(res)
}

//
// Internal Helpers
//

/// Record outbound HTTP metrics.
fn record_metrics(method: HttpMethod, url: &str, label: Option<&str>) {
    record_http_outcall();
    record_http_request(method, url, label);
}
