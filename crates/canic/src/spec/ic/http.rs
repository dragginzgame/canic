use candid::{CandidType, Deserialize, define_function};
use serde::Serialize;

pub type HeaderField = (String, String);

// Define the callback function type using the macro
define_function!(pub CallbackFunc : () -> () query);

///
/// HttpRequest
///

#[derive(CandidType, Clone, Deserialize)]
pub struct HttpRequest {
    pub url: String,
    pub method: String,
    pub headers: Vec<HeaderField>,
    pub body: Vec<u8>,
}

///
/// HttpResponse
///

#[derive(CandidType, Clone, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: Vec<HeaderField>,
    pub body: Vec<u8>,
    pub streaming_strategy: Option<StreamingStrategy>,
}

///
/// StreamingCallbackToken
///

#[derive(CandidType, Clone, Deserialize, Serialize)]
pub struct StreamingCallbackToken {
    pub asset_id: String,
    pub chunk_index: candid::Nat,
    pub headers: Vec<HeaderField>,
}

///
/// StreamingCallbackHttpResponse
///

#[derive(CandidType, Deserialize)]
pub struct StreamingCallbackHttpResponse {
    pub body: Vec<u8>,
    pub token: Option<StreamingCallbackToken>,
}

///
/// StreamingStrategy
///

#[derive(CandidType, Clone, Deserialize)]
pub enum StreamingStrategy {
    Callback {
        token: StreamingCallbackToken,
        callback: CallbackFunc,
    },
}
