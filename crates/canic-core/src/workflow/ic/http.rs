pub use crate::cdk::mgmt::{HttpHeader, HttpMethod, HttpRequestArgs, HttpRequestResult};

use crate::{
    Error,
    cdk::mgmt::http_request,
    model::metrics::system::{SystemMetricKind, SystemMetrics},
    ops::adapter::metrics::http::record_http_request,
};
use num_traits::ToPrimitive;
use serde::de::DeserializeOwned;

///
/// Http
///

pub struct Http;

impl Http {
    pub const MAX_RESPONSE_BYTES: u64 = 200_000;

    fn record_metrics(method: HttpMethod, url: &str, label: Option<&str>) {
        SystemMetrics::increment(SystemMetricKind::HttpOutcall);
        record_http_request(method, url, label);
    }

    pub async fn get<T: DeserializeOwned>(
        url: &str,
        headers: impl AsRef<[(&str, &str)]>,
    ) -> Result<T, Error> {
        Self::get_with_label(url, headers, None).await
    }

    pub async fn get_with_label<T: DeserializeOwned>(
        url: &str,
        headers: impl AsRef<[(&str, &str)]>,
        label: Option<&str>,
    ) -> Result<T, Error> {
        // metrics
        Self::record_metrics(HttpMethod::GET, url, label);

        let headers: Vec<HttpHeader> = headers
            .as_ref()
            .iter()
            .map(|(name, value)| HttpHeader {
                name: name.to_string(),
                value: value.to_string(),
            })
            .collect();

        let args = HttpRequestArgs {
            url: url.to_string(),
            method: HttpMethod::GET,
            headers,
            max_response_bytes: Some(Self::MAX_RESPONSE_BYTES),
            ..Default::default()
        };

        let res = http_request(&args)
            .await
            .map_err(|e| Error::HttpRequest(e.to_string()))?;

        let status: u32 = res.status.0.to_u32().unwrap_or(0);
        if !(200..300).contains(&status) {
            return Err(Error::HttpErrorCode(status));
        }

        serde_json::from_slice(&res.body).map_err(Into::into)
    }

    pub async fn get_raw(args: HttpRequestArgs) -> Result<HttpRequestResult, Error> {
        Self::get_raw_with_label(args, None).await
    }

    pub async fn get_raw_with_label(
        args: HttpRequestArgs,
        label: Option<&str>,
    ) -> Result<HttpRequestResult, Error> {
        // metrics
        Self::record_metrics(args.method, &args.url, label);

        http_request(&args)
            .await
            .map_err(|e| crate::Error::HttpRequest(e.to_string()))
    }
}
