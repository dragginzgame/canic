use crate::{
    PublicError,
    cdk::candid::{CandidType, Nat},
    dto,
    ops::ic::http::HttpOps,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

///
/// Http Api
///
/// Stable HTTP API for canic users.
/// Enforces metrics, limits, and IC-safe defaults.
///

///
/// HttpRequest
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct HttpRequest {
    pub url: String,
    pub max_response_bytes: Option<u64>,
    pub method: HttpMethod,
    pub headers: Vec<HttpHeader>,
    pub body: Option<Vec<u8>>,
    pub is_replicated: Option<bool>,
}

///
/// HttpResponse
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct HttpResponse {
    pub status: Nat,
    pub headers: Vec<HttpHeader>,
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
}

///
/// HttpMethod
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum HttpMethod {
    #[serde(rename = "get")]
    GET,
    #[serde(rename = "post")]
    POST,
    #[serde(rename = "head")]
    HEAD,
}

///
/// HttpHeader
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

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
pub async fn get_raw(args: HttpRequest) -> Result<HttpResponse, PublicError> {
    HttpOps::get_raw(args.into())
        .await
        .map(HttpResponse::from)
        .map_err(PublicError::from)
}

impl From<HttpRequest> for dto::http::HttpRequestArgs {
    fn from(req: HttpRequest) -> Self {
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

impl From<dto::http::HttpRequestResult> for HttpResponse {
    fn from(res: dto::http::HttpRequestResult) -> Self {
        Self {
            status: res.status,
            headers: res.headers.into_iter().map(Into::into).collect(),
            body: res.body,
        }
    }
}

impl From<HttpMethod> for dto::http::HttpMethod {
    fn from(method: HttpMethod) -> Self {
        match method {
            HttpMethod::GET => Self::GET,
            HttpMethod::POST => Self::POST,
            HttpMethod::HEAD => Self::HEAD,
        }
    }
}

impl From<HttpHeader> for dto::http::HttpHeader {
    fn from(header: HttpHeader) -> Self {
        Self {
            name: header.name,
            value: header.value,
        }
    }
}

impl From<dto::http::HttpHeader> for HttpHeader {
    fn from(header: dto::http::HttpHeader) -> Self {
        Self {
            name: header.name,
            value: header.value,
        }
    }
}
