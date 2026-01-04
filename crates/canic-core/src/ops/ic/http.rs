use crate::{Error, ThisError, dto, infra, ops};
use num_traits::ToPrimitive;
use serde::de::DeserializeOwned;

///
/// Http
/// Approved, observable HTTP helpers over the IC management API.
///

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
        ops::ic::IcOpsError::from(err).into()
    }
}

///
/// HttpOps
///

pub struct HttpOps;

impl HttpOps {
    /// Perform an HTTP GET request and deserialize the JSON response.
    pub async fn get<T: DeserializeOwned>(
        url: &str,
        headers: impl AsRef<[(&str, &str)]>,
    ) -> Result<T, Error> {
        Self::get_with_label(url, headers, None).await
    }

    /// Same as `get`, with an optional metrics label.
    pub async fn get_with_label<T: DeserializeOwned>(
        url: &str,
        headers: impl AsRef<[(&str, &str)]>,
        label: Option<&str>,
    ) -> Result<T, Error> {
        // Emit observability signals
        record_metrics(infra::ic::http::HttpMethod::GET, url, label);

        // Convert header pairs into IC HTTP headers
        let headers: Vec<infra::ic::http::HttpHeader> = headers
            .as_ref()
            .iter()
            .map(|(name, value)| infra::ic::http::HttpHeader {
                name: name.to_string(),
                value: value.to_string(),
            })
            .collect();

        // Build raw IC HTTP request arguments
        let args = infra::ic::http::HttpRequestArgs {
            url: url.to_string(),
            method: infra::ic::http::HttpMethod::GET,
            headers,
            max_response_bytes: Some(MAX_RESPONSE_BYTES),
            ..Default::default()
        };

        // Perform raw HTTP outcall via infra
        let res = infra::ic::http::http_request_raw(&args).await?;

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
    pub async fn get_raw(
        args: dto::http::HttpRequestArgs,
    ) -> Result<dto::http::HttpRequestResult, Error> {
        Self::get_raw_with_label(args, None).await
    }

    /// Same as `get_raw`, with an optional metrics label.
    pub async fn get_raw_with_label(
        args: dto::http::HttpRequestArgs,
        label: Option<&str>,
    ) -> Result<dto::http::HttpRequestResult, Error> {
        let infra_args = request_args_from_dto(args);

        // Emit observability signals
        record_metrics(infra_args.method, &infra_args.url, label);

        // Delegate to infra without additional interpretation
        let res = infra::ic::http::http_request_raw(&infra_args).await?;

        Ok(result_to_dto(res))
    }
}

//
// Internal helpers
//

/// Record outbound HTTP metrics.
fn record_metrics(method: infra::ic::http::HttpMethod, url: &str, label: Option<&str>) {
    ops::runtime::metrics::system::record_http_outcall();
    ops::runtime::metrics::http::record_http_request(metrics_method(method), url, label);
}

const fn metrics_method(
    method: infra::ic::http::HttpMethod,
) -> ops::runtime::metrics::http::HttpMethod {
    match method {
        infra::ic::http::HttpMethod::GET => ops::runtime::metrics::http::HttpMethod::Get,
        infra::ic::http::HttpMethod::POST => ops::runtime::metrics::http::HttpMethod::Post,
        infra::ic::http::HttpMethod::HEAD => ops::runtime::metrics::http::HttpMethod::Head,
    }
}

// -----------------------------------------------------------------------------
// DTO adapters
// -----------------------------------------------------------------------------

fn request_args_from_dto(args: dto::http::HttpRequestArgs) -> infra::ic::http::HttpRequestArgs {
    infra::ic::http::HttpRequestArgs {
        url: args.url,
        max_response_bytes: args.max_response_bytes,
        method: method_from_dto(args.method),
        headers: args.headers.into_iter().map(header_from_dto).collect(),
        body: args.body,
        transform: None,
        is_replicated: args.is_replicated,
    }
}

fn result_to_dto(result: infra::ic::http::HttpRequestResult) -> dto::http::HttpRequestResult {
    dto::http::HttpRequestResult {
        status: result.status,
        headers: result.headers.into_iter().map(header_to_dto).collect(),
        body: result.body,
    }
}

const fn method_from_dto(method: dto::http::HttpMethod) -> infra::ic::http::HttpMethod {
    match method {
        dto::http::HttpMethod::GET => infra::ic::http::HttpMethod::GET,
        dto::http::HttpMethod::POST => infra::ic::http::HttpMethod::POST,
        dto::http::HttpMethod::HEAD => infra::ic::http::HttpMethod::HEAD,
    }
}

fn header_from_dto(header: dto::http::HttpHeader) -> infra::ic::http::HttpHeader {
    infra::ic::http::HttpHeader {
        name: header.name,
        value: header.value,
    }
}

fn header_to_dto(header: infra::ic::http::HttpHeader) -> dto::http::HttpHeader {
    dto::http::HttpHeader {
        name: header.name,
        value: header.value,
    }
}
