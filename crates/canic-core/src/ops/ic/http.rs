use crate::{
    Error,
    // Raw IC HTTP passthrough (infra layer)
    infra::ic::http::{
        HttpHeader, HttpMethod, HttpRequestArgs, HttpRequestResult, http_request_raw,
    },
    // Observability (ops layer)
    ops::{adapter::metrics::http::record_http_request, runtime::metrics::record_http_outcall},
};
use num_traits::ToPrimitive;
use serde::de::DeserializeOwned;

///
/// Http
/// Approved, observable HTTP helpers over the IC management API.
///

pub struct Http;

impl Http {
    /// Maximum allowed response size for HTTP outcalls.
    pub const MAX_RESPONSE_BYTES: u64 = 200_000;

    //
    // Internal helpers
    //

    /// Record outbound HTTP metrics.
    fn record_metrics(method: HttpMethod, url: &str, label: Option<&str>) {
        record_http_outcall();
        record_http_request(method, url, label);
    }

    //
    // High-level typed helpers
    //

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
        Self::record_metrics(HttpMethod::GET, url, label);

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
            max_response_bytes: Some(Self::MAX_RESPONSE_BYTES),
            ..Default::default()
        };

        // Perform raw HTTP outcall via infra
        let res = http_request_raw(&args)
            .await
            .map_err(|e| Error::HttpRequest(e.to_string()))?;

        // Validate HTTP status code
        let status: u32 = res.status.0.to_u32().unwrap_or(0);
        if !(200..300).contains(&status) {
            return Err(Error::HttpErrorCode(status));
        }

        // Deserialize response body
        serde_json::from_slice(&res.body).map_err(Into::into)
    }

    //
    // Low-level escape hatches
    //

    /// Perform a raw HTTP request with metrics, returning the IC response verbatim.
    pub async fn get_raw(args: HttpRequestArgs) -> Result<HttpRequestResult, Error> {
        Self::get_raw_with_label(args, None).await
    }

    /// Same as `get_raw`, with an optional metrics label.
    pub async fn get_raw_with_label(
        args: HttpRequestArgs,
        label: Option<&str>,
    ) -> Result<HttpRequestResult, Error> {
        // Emit observability signals
        Self::record_metrics(args.method, &args.url, label);

        // Delegate to infra without additional interpretation
        http_request_raw(&args)
            .await
            .map_err(|e| Error::HttpRequest(e.to_string()))
    }
}
