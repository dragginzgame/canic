use crate::dto::prelude::*;
use candid::Nat;

///
/// HttpRequestArgs
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct HttpRequestArgs {
    pub url: String,
    pub max_response_bytes: Option<u64>,
    pub method: HttpMethod,
    pub headers: Vec<HttpHeader>,
    pub body: Option<Vec<u8>>,
    pub is_replicated: Option<bool>,
}

///
/// HttpRequestResult
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct HttpRequestResult {
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
