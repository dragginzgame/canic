pub mod adapter;

use crate::{Error, dto::http, ops::ic::http::HttpOps};
use serde::de::DeserializeOwned;

///
/// HttpWorkflow
///

pub struct HttpWorkflow;

impl HttpWorkflow {
    /// Perform a GET request and deserialize a JSON response.
    pub async fn get<T: DeserializeOwned>(url: &str, headers: &[(&str, &str)]) -> Result<T, Error> {
        HttpOps::get(url, headers).await
    }

    /// Same as `get`, with an explicit metrics label.
    pub async fn get_with_label<T: DeserializeOwned>(
        url: &str,
        headers: &[(&str, &str)],
        label: &str,
    ) -> Result<T, Error> {
        HttpOps::get_with_label(url, headers, Some(label)).await
    }

    pub async fn get_raw(args: http::HttpRequestArgs) -> Result<http::HttpRequestResult, Error> {
        Self::get_raw_with_label(args, None).await
    }

    pub async fn get_raw_with_label(
        args: http::HttpRequestArgs,
        label: Option<&str>,
    ) -> Result<http::HttpRequestResult, Error> {
        let infra_args = adapter::HttpAdapter::request_args_from_dto(args);
        let res = HttpOps::get_raw_with_label(infra_args, label).await?;

        Ok(adapter::HttpAdapter::result_to_dto(res))
    }
}
