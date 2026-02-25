use crate::{
    dto::{
        error::Error,
        http::{HttpRequestArgs, HttpRequestResult},
    },
    workflow::http::HttpWorkflow,
};

///
/// HttpApi
///
/// Stable HTTP API for canic users.
/// Enforces metrics, limits, and IC-safe defaults.
///

pub struct HttpApi;

impl HttpApi {
    /// Perform a GET request and return the raw response.
    /// Returns an error on non-2xx status codes.
    pub async fn get(url: &str, headers: &[(&str, &str)]) -> Result<HttpRequestResult, Error> {
        HttpWorkflow::get(url, headers).await.map_err(Error::from)
    }

    /// Same as `get`, with an explicit metrics label.
    /// Returns an error on non-2xx status codes.
    pub async fn get_with_label(
        url: &str,
        headers: &[(&str, &str)],
        label: &str,
    ) -> Result<HttpRequestResult, Error> {
        HttpWorkflow::get_with_label(url, headers, label)
            .await
            .map_err(Error::from)
    }

    /// Perform a raw HTTP request with metrics, returning the response verbatim.
    pub async fn get_raw(args: HttpRequestArgs) -> Result<HttpRequestResult, Error> {
        HttpWorkflow::get_raw(args).await.map_err(Error::from)
    }
}
