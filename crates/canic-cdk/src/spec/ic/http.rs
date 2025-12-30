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

impl HttpResponse {
    #[must_use]
    pub fn error(status: HttpStatus, message: &str) -> Self {
        Self {
            status_code: status.code(),
            headers: vec![("Content-Type".into(), "text/plain; charset=utf-8".into())],
            body: message.as_bytes().to_vec(),
            streaming_strategy: None,
        }
    }
}

///
/// HttpStatus
///

pub enum HttpStatus {
    Ok,
    BadRequest,
    NotFound,
    InternalError,
}

impl HttpStatus {
    #[must_use]
    pub const fn code(&self) -> u16 {
        match self {
            Self::Ok => 200,
            Self::BadRequest => 400,
            Self::NotFound => 404,
            Self::InternalError => 500,
        }
    }
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
