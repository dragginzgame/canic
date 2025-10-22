//! The [ICRC-21](https://github.com/dfinity/wg-identity-authentication/blob/main/topics/ICRC-21/icrc_21_consent_msg.md)
//! Canister Call Consent Messages standard.

///
/// NOTE: We're using the code directly as importing the icrc-ledger-types is not
/// currently a good idea.  It's got a lot of bloated dependencies and an older version
/// of ic-stable-structures
///
mod errors;

pub use errors::*;

use crate::spec::prelude::*;

///
/// ConsentInfo
///

#[derive(Debug, CandidType, Clone, Deserialize, PartialEq, Eq)]
pub struct ConsentInfo {
    pub consent_message: ConsentMessage,
    pub metadata: ConsentMessageMetadata,
}

///
/// ConsentMessage
///

#[derive(Debug, CandidType, Clone, Deserialize, PartialEq, Eq)]
pub enum ConsentMessage {
    GenericDisplayMessage(String),
    FieldsDisplayMessage(FieldsDisplay),
}

///
/// ConsentMessageMetadata
///

#[derive(Debug, CandidType, Clone, Deserialize, PartialEq, Eq)]
pub struct ConsentMessageMetadata {
    pub language: String,
    pub utc_offset_minutes: Option<i16>,
}

///
/// ConsentMessageRequest
///

#[derive(Debug, CandidType, Clone, Deserialize, PartialEq, Eq)]
pub struct ConsentMessageRequest {
    pub method: String,
    pub arg: Vec<u8>,
    pub user_preferences: ConsentMessageSpec,
}

///
/// ConsentMessageResponse
/// Wrapper capturing the ok/error variants from an ICRC-21 consent request.
///

#[derive(CandidType, Deserialize)]
pub enum ConsentMessageResponse {
    Ok(ConsentInfo),
    Err(Icrc21Error),
}

///
/// ConsentMessageSpec
///

#[derive(Debug, CandidType, Clone, Deserialize, PartialEq, Eq)]
pub struct ConsentMessageSpec {
    pub metadata: ConsentMessageMetadata,
    pub device_spec: Option<DisplayMessageType>,
}

///
/// DisplayMessageType
///

#[derive(Debug, CandidType, Clone, Deserialize, PartialEq, Eq)]
pub enum DisplayMessageType {
    GenericDisplay,
    FieldsDisplay,
}

///
/// FieldsDisplay
///

#[derive(Debug, CandidType, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct FieldsDisplay {
    pub intent: String,
    pub fields: Vec<(String, Value)>,
}

///
/// Value
///

#[derive(CandidType, Deserialize, Eq, PartialEq, Debug, Clone)]
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
