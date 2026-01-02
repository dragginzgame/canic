use crate::{
    Error, ThisError,
    dto::http::{
        HttpHeader as HttpHeaderDto, HttpMethod as HttpMethodDto,
        HttpRequestArgs as HttpRequestArgsDto, HttpRequestResult as HttpRequestResultDto,
    },
    infra::ic::http::{
        HttpHeader, HttpMethod, HttpRequestArgs, HttpRequestResult, http_request_raw,
    },
    ops::{
        ic::IcOpsError,
        runtime::metrics::{http::record_http_request, system::record_http_outcall},
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
        IcOpsError::from(err).into()
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
pub async fn get_raw(args: HttpRequestArgsDto) -> Result<HttpRequestResultDto, Error> {
    get_raw_with_label(args, None).await
}

/// Same as `get_raw`, with an optional metrics label.
pub async fn get_raw_with_label(
    args: HttpRequestArgsDto,
    label: Option<&str>,
) -> Result<HttpRequestResultDto, Error> {
    let infra_args = request_args_from_dto(args);

    // Emit observability signals
    record_metrics(infra_args.method, &infra_args.url, label);

    // Delegate to infra without additional interpretation
    let res = http_request_raw(&infra_args).await?;

    Ok(result_to_dto(res))
}

//
// Internal Helpers
//

/// Record outbound HTTP metrics.
fn record_metrics(method: HttpMethod, url: &str, label: Option<&str>) {
    record_http_outcall();
    record_http_request(method, url, label);
}

// --- DTO Adapters --------------------------------------------------------

fn request_args_from_dto(args: HttpRequestArgsDto) -> HttpRequestArgs {
    HttpRequestArgs {
        url: args.url,
        max_response_bytes: args.max_response_bytes,
        method: method_from_dto(args.method),
        headers: args.headers.into_iter().map(header_from_dto).collect(),
        body: args.body,
        transform: None,
        is_replicated: args.is_replicated,
    }
}

fn result_to_dto(result: HttpRequestResult) -> HttpRequestResultDto {
    HttpRequestResultDto {
        status: result.status,
        headers: result.headers.into_iter().map(header_to_dto).collect(),
        body: result.body,
    }
}

const fn method_from_dto(method: HttpMethodDto) -> HttpMethod {
    match method {
        HttpMethodDto::GET => HttpMethod::GET,
        HttpMethodDto::POST => HttpMethod::POST,
        HttpMethodDto::HEAD => HttpMethod::HEAD,
    }
}

fn header_from_dto(header: HttpHeaderDto) -> HttpHeader {
    HttpHeader {
        name: header.name,
        value: header.value,
    }
}

fn header_to_dto(header: HttpHeader) -> HttpHeaderDto {
    HttpHeaderDto {
        name: header.name,
        value: header.value,
    }
}
