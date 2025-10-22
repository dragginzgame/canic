use crate::spec::prelude::*;
use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// ConsentMessageResponse
/// Wrapper capturing the ok/error variants from an ICRC-21 consent request.
///

#[derive(CandidType, Deserialize)]
pub enum ConsentMessageResponse {
    Ok(ConsentInfo),
    Err(Icrc21Error),
}

// ---------------------------------- COPY AND PASTE CODE FOR NOW ----------------------

#[derive(Debug, CandidType, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorInfo {
    pub description: String,
}

#[derive(Debug, CandidType, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Icrc21Error {
    UnsupportedCanisterCall(ErrorInfo),
    ConsentMessageUnavailable(ErrorInfo),
    InsufficientPayment(ErrorInfo),
    GenericError {
        error_code: Nat,
        description: String,
    },
}

#[derive(Debug, CandidType, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsentMessageMetadata {
    pub language: String,
    pub utc_offset_minutes: Option<i16>,
}

#[derive(Debug, CandidType, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DisplayMessageType {
    GenericDisplay,
    FieldsDisplay,
}

#[derive(Debug, CandidType, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsentMessageSpec {
    pub metadata: ConsentMessageMetadata,
    pub device_spec: Option<DisplayMessageType>,
}

#[derive(Debug, CandidType, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsentMessageRequest {
    pub method: String,
    pub arg: Vec<u8>,
    pub user_preferences: ConsentMessageSpec,
}

#[derive(Debug, CandidType, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsentInfo {
    pub consent_message: ConsentMessage,
    pub metadata: ConsentMessageMetadata,
}

#[derive(Debug, CandidType, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsentMessage {
    GenericDisplayMessage(String),
    FieldsDisplayMessage(FieldsDisplay),
}

#[derive(Debug, CandidType, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FieldsDisplay {
    pub intent: String,
    pub fields: Vec<(String, Value)>,
}

#[derive(CandidType, Deserialize, Eq, PartialEq, Debug, Serialize, Clone)]
pub enum Value {
    TokenAmount {
        decimals: u8,
        amount: u64,
        symbol: String,
    },
    TimestampSeconds {
        amount: u64,
    },
    DurationSeconds {
        amount: u64,
    },
    Text {
        content: String,
    },
}
