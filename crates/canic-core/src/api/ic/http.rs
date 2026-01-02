use crate::{
    PublicError,
    cdk::mgmt::{HttpRequestArgs, HttpRequestResult},
    ops::ic::http as http_ops,
};
use serde::de::DeserializeOwned;

///
/// Http Api
///
/// Stable HTTP API for canic users.
/// Enforces metrics, limits, and IC-safe defaults.
///

/// Perform a GET request and deserialize a JSON response.
pub async fn get<T: DeserializeOwned>(
    url: &str,
    headers: &[(&str, &str)],
) -> Result<T, PublicError> {
    http_ops::get(url, headers).await.map_err(PublicError::from)
}

/// Same as `get`, with an explicit metrics label.
pub async fn get_with_label<T: DeserializeOwned>(
    url: &str,
    headers: &[(&str, &str)],
    label: &str,
) -> Result<T, PublicError> {
    http_ops::get_with_label(url, headers, Some(label))
        .await
        .map_err(PublicError::from)
}

/// Perform a raw HTTP GET with metrics, returning the IC response verbatim.
pub async fn get_raw(args: HttpRequestArgs) -> Result<HttpRequestResult, PublicError> {
    http_ops::get_raw(args).await.map_err(PublicError::from)
}
