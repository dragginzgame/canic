use crate::{
    InternalError, ThisError,
    ids::SystemMetricKind,
    infra::{
        InfraError,
        ic::http::{HttpHeader, HttpInfra, HttpMethod, HttpRequestArgs, HttpRequestResult},
    },
    ops::{
        ic::IcOpsError,
        runtime::metrics::{http::HttpMetrics, system::SystemMetrics},
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
    Infra(#[from] InfraError),

    #[error(transparent)]
    HttpDecode(#[from] serde_json::Error),
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

    /// Perform an HTTP GET request and deserialize the JSON response.
    pub async fn get<T: DeserializeOwned>(
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<T, InternalError> {
        Self::get_with_label(url, headers, None).await
    }

    /// Same as `get`, with an optional metrics label.
    pub async fn get_with_label<T: DeserializeOwned>(
        url: &str,
        headers: &[(&str, &str)],
        label: Option<&str>,
    ) -> Result<T, InternalError> {
        let args = HttpRequestArgs {
            url: url.to_string(),
            method: HttpMethod::GET,
            headers: Self::headers_from_pairs(headers),
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
        let res = HttpInfra::http_request_raw(&args)
            .await
            .map_err(HttpOpsError::from)?;

        Ok(res)
    }

    /// Record outbound HTTP metrics.
    fn record_metrics(method: HttpMethod, url: &str, label: Option<&str>) {
        SystemMetrics::increment(SystemMetricKind::HttpOutcall);
        HttpMetrics::record_http_request(method.into(), url, label);
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
