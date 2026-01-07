use crate::{PublicError, dto, ops::ic::http::HttpOps, workflow::http::HttpWorkflow};
use serde::de::DeserializeOwned;

///
/// HttpApi
///
/// Stable HTTP API for canic users.
/// Enforces metrics, limits, and IC-safe defaults.
///

pub struct HttpApi;

impl HttpApi {
    /// Perform a GET request and deserialize a JSON response.
    /// Returns an error on non-2xx status codes or JSON decode failures.
    pub async fn get<T: DeserializeOwned>(
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<T, PublicError> {
        HttpOps::get(url, headers).await.map_err(PublicError::from)
    }

    /// Same as `get`, with an explicit metrics label.
    /// Returns an error on non-2xx status codes or JSON decode failures.
    pub async fn get_with_label<T: DeserializeOwned>(
        url: &str,
        headers: &[(&str, &str)],
        label: &str,
    ) -> Result<T, PublicError> {
        HttpOps::get_with_label(url, headers, Some(label))
            .await
            .map_err(PublicError::from)
    }

    /// Perform a raw HTTP request with metrics, returning the response verbatim.
    pub async fn get_raw(
        args: dto::http::HttpRequestArgs,
    ) -> Result<dto::http::HttpRequestResult, PublicError> {
        HttpWorkflow::get_raw(args).await.map_err(PublicError::from)
    }
}
