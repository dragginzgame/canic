pub mod adapter;

use crate::{InternalError, dto::http, ops::ic::http::HttpOps};

///
/// HttpWorkflow
///

pub struct HttpWorkflow;

impl HttpWorkflow {
    /// Perform a GET request and return the raw response.
    pub async fn get(
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<http::HttpRequestResult, InternalError> {
        let res = HttpOps::get(url, headers).await?;
        Ok(adapter::HttpAdapter::result_to_dto(res))
    }

    /// Same as `get`, with an explicit metrics label.
    pub async fn get_with_label(
        url: &str,
        headers: &[(&str, &str)],
        label: &str,
    ) -> Result<http::HttpRequestResult, InternalError> {
        let res = HttpOps::get_with_label(url, headers, Some(label)).await?;
        Ok(adapter::HttpAdapter::result_to_dto(res))
    }

    pub async fn get_raw(
        args: http::HttpRequestArgs,
    ) -> Result<http::HttpRequestResult, InternalError> {
        Self::get_raw_with_label(args, None).await
    }

    pub async fn get_raw_with_label(
        args: http::HttpRequestArgs,
        label: Option<&str>,
    ) -> Result<http::HttpRequestResult, InternalError> {
        let infra_args = adapter::HttpAdapter::request_args_from_dto(args);
        let res = HttpOps::get_raw_with_label(infra_args, label).await?;

        Ok(adapter::HttpAdapter::result_to_dto(res))
    }
}
