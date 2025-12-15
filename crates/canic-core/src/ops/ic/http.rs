#![allow(clippy::disallowed_methods)]

use crate::{
    Error,
    cdk::mgmt::{HttpHeader, HttpMethod, HttpRequestArgs, http_request},
    model::metrics::{
        http::HttpMetrics,
        system::{SystemMetricKind, SystemMetrics},
    },
};
use num_traits::ToPrimitive;
use serde::de::DeserializeOwned;

const MAX_RESPONSE_BYTES: u64 = 200_000;

///
/// http_get
/// Generic helper for HTTP GET with JSON response.
///
pub async fn http_get<T: DeserializeOwned>(
    url: &str,
    headers: &[(String, String)],
) -> Result<T, Error> {
    http_get_with_label(url, headers, None).await
}

/// http_get_with_label
/// HTTP GET with optional stable metric label.
pub async fn http_get_with_label<T: DeserializeOwned>(
    url: &str,
    headers: &[(String, String)],
    label: Option<&str>,
) -> Result<T, Error> {
    // record metrics up front so attempts are counted
    SystemMetrics::increment(SystemMetricKind::HttpOutcall);
    HttpMetrics::increment_with_label("GET", url, label);

    let headers: Vec<HttpHeader> = headers
        .iter()
        .map(|(name, value)| HttpHeader {
            name: name.clone(),
            value: value.clone(),
        })
        .collect();

    let args = HttpRequestArgs {
        url: url.to_string(),
        method: HttpMethod::GET,
        headers,
        max_response_bytes: Some(MAX_RESPONSE_BYTES),
        ..Default::default()
    };

    let res = http_request(&args)
        .await
        .map_err(|e| Error::HttpRequest(e.to_string()))?;

    // status
    let status: u32 = res.status.0.to_u32().unwrap_or(0);
    if status != 200 {
        return Err(Error::HttpErrorCode(status));
    }

    // deserialize json
    let res: T = serde_json::from_slice::<T>(&res.body)?;

    Ok(res)
}
