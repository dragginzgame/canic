pub mod adapter;

use crate::{Error, dto::http, ops::ic::http::HttpOps};

///
/// HttpWorkflow
///

pub struct HttpWorkflow;

impl HttpWorkflow {
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
