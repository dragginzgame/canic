use crate::dto::prelude::*;

pub use crate::domain::http::HttpMethod;

//
// HttpRequestArgs
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct HttpRequestArgs {
    pub url: String,
    pub max_response_bytes: Option<u64>,
    pub method: HttpMethod,
    pub headers: Vec<HttpHeader>,
    pub body: Option<Vec<u8>>,
    pub is_replicated: Option<bool>,
}

//
// HttpRequestResult
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct HttpRequestResult {
    pub status: Nat,
    pub headers: Vec<HttpHeader>,
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
}

//
// HttpHeader
//

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{CandidType, Decode, Encode};
    use serde::Deserialize;

    #[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
    enum LegacyHttpMethod {
        #[serde(rename = "get")]
        GET,
        #[serde(rename = "post")]
        POST,
        #[serde(rename = "head")]
        HEAD,
    }

    #[test]
    fn http_method_roundtrips_candid_through_dto_path() {
        let bytes = Encode!(&HttpMethod::Get).expect("encode domain http method");
        let legacy = Decode!(&bytes, LegacyHttpMethod).expect("decode legacy http method");

        assert_eq!(legacy, LegacyHttpMethod::GET);

        let legacy_bytes = Encode!(&LegacyHttpMethod::HEAD).expect("encode legacy http method");
        let method = Decode!(&legacy_bytes, HttpMethod).expect("decode domain http method");

        assert_eq!(method, HttpMethod::Head);
    }
}
