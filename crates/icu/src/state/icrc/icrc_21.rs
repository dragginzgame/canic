use candid::CandidType;
use derive_more::{Deref, DerefMut};
use serde::Deserialize;
use std::{cell::RefCell, collections::HashMap};

//
// ICRC 21 REGISTRY
//

thread_local! {
    static ICRC_21_REGISTRY: RefCell<Icrc21Registry> = RefCell::new(Icrc21Registry::new());
}

///
/// ConsentHandlerFn
/// this is what the user has to pass into icu
///

pub type Icrc21ConsentHandlerFn = fn(
    request: Icrc21ConsentMessageRequest,
) -> Result<Option<Icrc21ConsentMessageResponse>, String>;

///

///
/// Icrc21 ConsentMessage
///

#[derive(CandidType, Deserialize)]
pub enum Icrc21ConsentMessage {
    GenericDisplayMessage(String),
    FieldsDisplayMessage {
        intent: String,
        fields: Vec<(String, Icrc21Value)>,
    },
}

#[derive(CandidType, Deserialize, Clone)]
pub struct Icrc21ConsentMessageSpec {
    pub metadata: Icrc21ConsentMessageMetadata,
    pub device_spec: Option<Icrc21DeviceSpec>,
}

#[derive(CandidType, Deserialize)]
pub struct Icrc21ConsentMessageRequest {
    pub method: String,
    pub arg: Vec<u8>,
    pub user_preferences: Icrc21ConsentPreferences,
}

#[derive(CandidType, Deserialize)]
pub enum Icrc21ConsentMessageResponse {
    Ok(Icrc21ConsentInfo),
    Err(Icrc21Error),
}

#[derive(CandidType, Deserialize)]
pub struct Icrc21ConsentInfo {
    pub consent_message: Icrc21ConsentMessage,
    pub metadata: Icrc21ConsentMessageMetadata,
}

#[derive(CandidType, Deserialize, Clone)]
pub struct Icrc21ConsentMessageMetadata {
    pub language: String,
    pub utc_offset_minutes: Option<i16>,
}

///
/// Icrc21ConsertPreferences
///

#[derive(CandidType, Deserialize, Clone)]
pub struct Icrc21ConsentPreferences {
    pub language: String,
}

#[derive(CandidType, Deserialize, Clone)]
pub enum Icrc21DeviceSpec {
    GenericDisplay,
    FieldsDisplay,
}

///
/// Icrc21 Values
///

#[derive(CandidType, Deserialize)]
pub enum Icrc21Value {
    TokenAmount(Icrc21TokenAmount),
    TimestampSeconds(Icrc21TimestampSeconds),
    DurationSeconds(Icrc21DurationSeconds),
    Text(Icrc21TextValue),
}

#[derive(CandidType, Deserialize)]
pub struct Icrc21TokenAmount {
    pub decimals: u8,
    pub amount: u64,
    pub symbol: String,
}

#[derive(CandidType, Deserialize)]
pub struct Icrc21TimestampSeconds {
    pub amount: u64,
}

#[derive(CandidType, Deserialize)]
pub struct Icrc21DurationSeconds {
    pub amount: u64,
}

#[derive(CandidType, Deserialize)]
pub struct Icrc21TextValue {
    pub content: String,
}

///
/// Icrc21Error
///

#[derive(CandidType, Deserialize)]
pub struct Icrc21ErrorInfo {
    pub description: String,
}

#[derive(CandidType, Deserialize)]
pub enum Icrc21Error {
    UnsupportedCanisterCall(Icrc21ErrorInfo),
    ConsentMessageUnavailable(Icrc21ErrorInfo),
    InsufficientPayment(Icrc21ErrorInfo),
    GenericError {
        error_code: u64,
        description: String,
    },
}

///
/// Icrc21Registry
///

#[derive(Default, Debug, Deref, DerefMut)]
pub struct Icrc21Registry(pub HashMap<String, Icrc21ConsentHandlerFn>);

impl Icrc21Registry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(method: &str, handler: Icrc21ConsentHandlerFn) {
        ICRC_21_REGISTRY.with_borrow_mut(|reg| reg.insert(method.to_string(), handler));
    }

    #[must_use]
    pub fn get_handler(method: &str) -> Option<Icrc21ConsentHandlerFn> {
        ICRC_21_REGISTRY.with_borrow(|reg| reg.get(method).copied())
    }

    #[must_use]
    pub fn consent_message(req: Icrc21ConsentMessageRequest) -> Icrc21ConsentMessageResponse {
        match Self::get_handler(&req.method) {
            Some(handler) => match handler(req) {
                Ok(Some(response)) => response,

                Ok(None) => Icrc21ConsentMessageResponse::Err(
                    Icrc21Error::ConsentMessageUnavailable(Icrc21ErrorInfo {
                        description: "No consent message available.".to_string(),
                    }),
                ),
                Err(desc) => Icrc21ConsentMessageResponse::Err(Icrc21Error::GenericError {
                    error_code: 1,
                    description: desc,
                }),
            },
            None => Icrc21ConsentMessageResponse::Err(Icrc21Error::UnsupportedCanisterCall(
                Icrc21ErrorInfo {
                    description: "No handler registered for this method.".to_string(),
                },
            )),
        }
    }
}
