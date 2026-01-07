use crate::{
    Error, ThisError, dto, infra,
    ops::{
        self,
        ic::IcOpsError,
        runtime::metrics::{
            http::HttpMetrics,
            system::{SystemMetricKind, SystemMetrics},
        },
    },
};
use num_traits::cast::ToPrimitive;
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

    #[error(transparent)]
    Infra(#[from] infra::InfraError),

    #[error(transparent)]
    HttpDecode(#[from] serde_json::Error),
}

impl From<HttpOpsError> for Error {
    fn from(err: HttpOpsError) -> Self {
        Self::from(IcOpsError::from(err))
    }
}

///
/// HttpOps
///

pub struct HttpOps;

impl HttpOps {
    // -------------------------------------------------------------------------
    // High-level helpers
    // -------------------------------------------------------------------------

    /// Perform an HTTP GET request and deserialize the JSON response.
    pub async fn get<T: DeserializeOwned>(url: &str, headers: &[(&str, &str)]) -> Result<T, Error> {
        Self::get_with_label(url, headers, None).await
    }

    /// Same as `get`, with an optional metrics label.
    pub async fn get_with_label<T: DeserializeOwned>(
        url: &str,
        headers: &[(&str, &str)],
        label: Option<&str>,
    ) -> Result<T, Error> {
        let args = infra::ic::http::HttpRequestArgs {
            url: url.to_string(),
            method: infra::ic::http::HttpMethod::GET,
            headers: headers_from_pairs(headers),
            max_response_bytes: Some(MAX_RESPONSE_BYTES),
            ..Default::default()
        };

        let res = Self::perform_request(args, label).await?;
        let status = res.status.0.to_u32().unwrap_or(u32::MAX);

        if !(200..300).contains(&status) {
            return Err(HttpOpsError::HttpStatus(status).into());
        }

        serde_json::from_slice(&res.body).map_err(|err| HttpOpsError::HttpDecode(err).into())
    }

    // -------------------------------------------------------------------------
    // Low-level escape hatches
    // -------------------------------------------------------------------------

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
        let res = Self::perform_request(infra_args, label).await?;
        Ok(result_to_dto(res))
    }

    // -------------------------------------------------------------------------
    // Core execution
    // -------------------------------------------------------------------------

    /// Perform a raw IC HTTP outcall with mandatory metrics.
    async fn perform_request(
        args: infra::ic::http::HttpRequestArgs,
        label: Option<&str>,
    ) -> Result<infra::ic::http::HttpRequestResult, Error> {
        Self::record_metrics(args.method, &args.url, label);
        let res = infra::ic::http::http_request_raw(&args)
            .await
            .map_err(HttpOpsError::from)?;

        Ok(res)
    }

    /// Record outbound HTTP metrics.
    fn record_metrics(method: infra::ic::http::HttpMethod, url: &str, label: Option<&str>) {
        SystemMetrics::increment(SystemMetricKind::HttpOutcall);
        HttpMetrics::record_http_request(metrics_method(method), url, label);
    }
}

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

fn headers_from_pairs(headers: &[(&str, &str)]) -> Vec<infra::ic::http::HttpHeader> {
    headers
        .iter()
        .map(|(name, value)| infra::ic::http::HttpHeader {
            name: (*name).to_string(),
            value: (*value).to_string(),
        })
        .collect()
}

// -----------------------------------------------------------------------------
// Infra adapters
// -----------------------------------------------------------------------------

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
