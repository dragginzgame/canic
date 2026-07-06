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
    use candid::{Decode, Encode};

    #[test]
    fn http_method_roundtrips_candid_with_canonical_labels() {
        let bytes = Encode!(&HttpMethod::Get).expect("encode domain http method");
        let method = Decode!(&bytes, HttpMethod).expect("decode domain http method");

        assert_eq!(method, HttpMethod::Get);
    }

    #[test]
    fn http_method_deserializes_canonical_lowercase_labels() {
        assert_http_method_label(HttpMethod::Get, "get");
        assert_http_method_label(HttpMethod::Head, "head");
        assert_http_method_label(HttpMethod::Post, "post");
    }

    fn assert_http_method_label(method: HttpMethod, label: &str) {
        let label_bytes = serde_cbor::to_vec(&label).expect("encode HTTP method label");
        let decoded: HttpMethod =
            serde_cbor::from_slice(&label_bytes).expect("decode HTTP method label");
        assert_eq!(decoded, method);
    }
}
