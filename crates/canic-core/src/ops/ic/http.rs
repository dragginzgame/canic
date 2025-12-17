pub use crate::cdk::mgmt::{HttpHeader, HttpMethod, HttpRequestArgs, HttpRequestResult};

use crate::{
    Error,
    cdk::mgmt::http_request,
    model::metrics::{
        http::HttpMetrics,
        system::{SystemMetricKind, SystemMetrics},
    },
};
use num_traits::ToPrimitive;
use serde::de::DeserializeOwned;

///
/// Http
///

pub struct Http;

impl Http {
    pub const MAX_RESPONSE_BYTES: u64 = 200_000;

    fn record_metrics(verb: &'static str, url: &str, label: Option<&str>) {
        SystemMetrics::increment(SystemMetricKind::HttpOutcall);
        HttpMetrics::increment_with_label(verb, url, label);
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
        Self::record_metrics("GET", url, label);

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
        if status != 200 {
            return Err(Error::HttpErrorCode(status));
        }

        serde_json::from_slice(&res.body).map_err(Into::into)
    }

    pub async fn get_raw<T: DeserializeOwned>(
        args: HttpRequestArgs,
    ) -> Result<HttpRequestResult, Error> {
        Self::get_raw_with_label(args, None).await
    }

    pub async fn get_raw_with_label(
        args: HttpRequestArgs,
        label: Option<&str>,
    ) -> Result<HttpRequestResult, Error> {
        // metrics
        Self::record_metrics("GET", &args.url, label);

        http_request(&args)
            .await
            .map_err(|e| crate::Error::HttpRequest(e.to_string()))
    }
}
