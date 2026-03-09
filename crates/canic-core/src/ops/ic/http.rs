use crate::{
    InternalError,
    ids::SystemMetricKind,
    infra::{InfraError, ic::http::HttpInfra},
    ops::{
        ic::IcOpsError,
        runtime::metrics::{
            http::{HttpMethod as MetricsHttpMethod, HttpMetrics},
            system::SystemMetrics,
        },
    },
};
use candid::Nat;
use thiserror::Error as ThisError;

///
/// Http
/// Approved, observable HTTP helpers over the IC management API.
///

/// Maximum allowed response size for HTTP outcalls.
pub const MAX_RESPONSE_BYTES: u64 = 200_000;

///
/// HttpHeader
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

///
/// HttpMethod
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpMethod {
    Get,
    Head,
    Post,
}

///
/// HttpRequestArgs
///

#[derive(Clone, Debug)]
pub struct HttpRequestArgs {
    pub url: String,
    pub max_response_bytes: Option<u64>,
    pub method: HttpMethod,
    pub headers: Vec<HttpHeader>,
    pub body: Option<Vec<u8>>,
    pub is_replicated: Option<bool>,
}

impl Default for HttpRequestArgs {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_response_bytes: None,
            method: HttpMethod::Get,
            headers: Vec::new(),
            body: None,
            is_replicated: None,
        }
    }
}

///
/// HttpRequestResult
///

#[derive(Clone, Debug)]
pub struct HttpRequestResult {
    pub status: Nat,
    pub headers: Vec<HttpHeader>,
    pub body: Vec<u8>,
}

///
/// HttpOpsError
///

#[derive(Debug, ThisError)]
pub enum HttpOpsError {
    #[error("http error status: {0}")]
    HttpStatus(u32),

    #[error(transparent)]
    Infra(#[from] InfraError),
}

impl From<HttpOpsError> for InternalError {
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

    /// Perform an HTTP GET request and return the raw response.
    pub async fn get(
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<HttpRequestResult, InternalError> {
        Self::get_with_label(url, headers, None).await
    }

    /// Same as `get`, with an optional metrics label.
    pub async fn get_with_label(
        url: &str,
        headers: &[(&str, &str)],
        label: Option<&str>,
    ) -> Result<HttpRequestResult, InternalError> {
        let args = HttpRequestArgs {
            url: url.to_string(),
            method: HttpMethod::Get,
            headers: Self::headers_from_pairs(headers),
            max_response_bytes: Some(MAX_RESPONSE_BYTES),
            ..Default::default()
        };

        let res = Self::perform_request(args, label).await?;
        let status = u32::try_from(&res.status.0).unwrap_or(u32::MAX);

        if !(200..300).contains(&status) {
            return Err(HttpOpsError::HttpStatus(status).into());
        }

        Ok(res)
    }

    // -------------------------------------------------------------------------
    // Low-level escape hatches
    // -------------------------------------------------------------------------

    /// Same as `get_raw`, with an optional metrics label.
    pub async fn get_raw_with_label(
        args: HttpRequestArgs,
        label: Option<&str>,
    ) -> Result<HttpRequestResult, InternalError> {
        Self::perform_request(args, label).await
    }

    // -------------------------------------------------------------------------
    // Core execution
    // -------------------------------------------------------------------------

    /// Perform a raw IC HTTP outcall with mandatory metrics.
    async fn perform_request(
        args: HttpRequestArgs,
        label: Option<&str>,
    ) -> Result<HttpRequestResult, InternalError> {
        Self::record_metrics(args.method, &args.url, label);
        let cdk_args = crate::cdk::mgmt::HttpRequestArgs::from(args);
        let res = HttpInfra::http_request_raw(&cdk_args)
            .await
            .map_err(HttpOpsError::from)?;

        Ok(HttpRequestResult::from(res))
    }

    /// Record outbound HTTP metrics.
    fn record_metrics(method: HttpMethod, url: &str, label: Option<&str>) {
        SystemMetrics::increment(SystemMetricKind::HttpOutcall);
        HttpMetrics::record_http_request(metrics_method(method), url, label);
    }

    ///
    /// helpers
    ///

    fn headers_from_pairs(headers: &[(&str, &str)]) -> Vec<HttpHeader> {
        headers
            .iter()
            .map(|(name, value)| HttpHeader {
                name: (*name).to_string(),
                value: (*value).to_string(),
            })
            .collect()
    }
}

const fn metrics_method(method: HttpMethod) -> MetricsHttpMethod {
    match method {
        HttpMethod::Get => MetricsHttpMethod::Get,
        HttpMethod::Post => MetricsHttpMethod::Post,
        HttpMethod::Head => MetricsHttpMethod::Head,
    }
}

impl From<HttpMethod> for crate::cdk::mgmt::HttpMethod {
    fn from(method: HttpMethod) -> Self {
        match method {
            HttpMethod::Get => Self::GET,
            HttpMethod::Post => Self::POST,
            HttpMethod::Head => Self::HEAD,
        }
    }
}

impl From<crate::cdk::mgmt::HttpMethod> for HttpMethod {
    fn from(method: crate::cdk::mgmt::HttpMethod) -> Self {
        match method {
            crate::cdk::mgmt::HttpMethod::GET => Self::Get,
            crate::cdk::mgmt::HttpMethod::POST => Self::Post,
            crate::cdk::mgmt::HttpMethod::HEAD => Self::Head,
        }
    }
}

impl From<HttpHeader> for crate::cdk::mgmt::HttpHeader {
    fn from(header: HttpHeader) -> Self {
        Self {
            name: header.name,
            value: header.value,
        }
    }
}

impl From<crate::cdk::mgmt::HttpHeader> for HttpHeader {
    fn from(header: crate::cdk::mgmt::HttpHeader) -> Self {
        Self {
            name: header.name,
            value: header.value,
        }
    }
}

impl From<HttpRequestArgs> for crate::cdk::mgmt::HttpRequestArgs {
    fn from(args: HttpRequestArgs) -> Self {
        Self {
            url: args.url,
            max_response_bytes: args.max_response_bytes,
            method: args.method.into(),
            headers: args.headers.into_iter().map(Into::into).collect(),
            body: args.body,
            transform: None,
            is_replicated: args.is_replicated,
        }
    }
}

impl From<crate::cdk::mgmt::HttpRequestResult> for HttpRequestResult {
    fn from(result: crate::cdk::mgmt::HttpRequestResult) -> Self {
        Self {
            status: result.status,
            headers: result.headers.into_iter().map(Into::into).collect(),
            body: result.body,
        }
    }
}
