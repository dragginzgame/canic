use crate::{PublicError, dto::http as http_dto, ops::ic::http as http_ops};
use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

///
/// Http Api
///
/// Stable HTTP API for canic users.
/// Enforces metrics, limits, and IC-safe defaults.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ApiHttpRequest {
    pub url: String,
    pub max_response_bytes: Option<u64>,
    pub method: ApiHttpMethod,
    pub headers: Vec<ApiHttpHeader>,
    pub body: Option<Vec<u8>>,
    pub is_replicated: Option<bool>,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ApiHttpResponse {
    pub status: Nat,
    pub headers: Vec<ApiHttpHeader>,
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ApiHttpMethod {
    #[serde(rename = "get")]
    GET,
    #[serde(rename = "post")]
    POST,
    #[serde(rename = "head")]
    HEAD,
}

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ApiHttpHeader {
    pub name: String,
    pub value: String,
}

/// Perform a GET request and deserialize a JSON response.
/// Returns an error on non-2xx status codes or JSON decode failures.
pub async fn get<T: DeserializeOwned>(
    url: &str,
    headers: &[(&str, &str)],
) -> Result<T, PublicError> {
    http_ops::get(url, headers).await.map_err(PublicError::from)
}

/// Same as `get`, with an explicit metrics label.
/// Returns an error on non-2xx status codes or JSON decode failures.
pub async fn get_with_label<T: DeserializeOwned>(
    url: &str,
    headers: &[(&str, &str)],
    label: &str,
) -> Result<T, PublicError> {
    http_ops::get_with_label(url, headers, Some(label))
        .await
        .map_err(PublicError::from)
}

/// Perform a raw HTTP request with metrics, returning the response verbatim.
pub async fn get_raw(args: ApiHttpRequest) -> Result<ApiHttpResponse, PublicError> {
    http_ops::get_raw(args.into())
        .await
        .map(ApiHttpResponse::from)
        .map_err(PublicError::from)
}

impl From<ApiHttpRequest> for http_dto::HttpRequestArgs {
    fn from(req: ApiHttpRequest) -> Self {
        Self {
            url: req.url,
            max_response_bytes: req.max_response_bytes,
            method: req.method.into(),
            headers: req.headers.into_iter().map(Into::into).collect(),
            body: req.body,
            is_replicated: req.is_replicated,
        }
    }
}

impl From<http_dto::HttpRequestResult> for ApiHttpResponse {
    fn from(res: http_dto::HttpRequestResult) -> Self {
        Self {
            status: res.status,
            headers: res.headers.into_iter().map(Into::into).collect(),
            body: res.body,
        }
    }
}

impl From<ApiHttpMethod> for http_dto::HttpMethod {
    fn from(method: ApiHttpMethod) -> Self {
        match method {
            ApiHttpMethod::GET => Self::GET,
            ApiHttpMethod::POST => Self::POST,
            ApiHttpMethod::HEAD => Self::HEAD,
        }
    }
}

impl From<ApiHttpHeader> for http_dto::HttpHeader {
    fn from(header: ApiHttpHeader) -> Self {
        Self {
            name: header.name,
            value: header.value,
        }
    }
}

impl From<http_dto::HttpHeader> for ApiHttpHeader {
    fn from(header: http_dto::HttpHeader) -> Self {
        Self {
            name: header.name,
            value: header.value,
        }
    }
}
