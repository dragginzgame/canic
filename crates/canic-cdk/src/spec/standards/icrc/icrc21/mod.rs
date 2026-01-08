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
/// ConsentResult
/// (type alias)
///

pub type ConsentResult = Result<ConsentInfo, Icrc21Error>;

///
/// ConsentInfo
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ConsentInfo {
    pub consent_message: ConsentMessage,
    pub metadata: ConsentMessageMetadata,
}

///
/// ConsentMessage
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum ConsentMessage {
    GenericDisplayMessage(String),
    FieldsDisplayMessage(FieldsDisplay),
}

///
/// ConsentMessageMetadata
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ConsentMessageMetadata {
    pub language: String,
    pub utc_offset_minutes: Option<i16>,
}

///
/// ConsentMessageRequest
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
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

impl From<ConsentResult> for ConsentMessageResponse {
    fn from(res: ConsentResult) -> Self {
        match res {
            Ok(info) => Self::Ok(info),
            Err(err) => Self::Err(err),
        }
    }
}

///
/// ConsentMessageSpec
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ConsentMessageSpec {
    pub metadata: ConsentMessageMetadata,
    pub device_spec: Option<DisplayMessageType>,
}

///
/// DisplayMessageType
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum DisplayMessageType {
    GenericDisplay,
    FieldsDisplay,
}

///
/// FieldsDisplay
///

#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct FieldsDisplay {
    pub intent: String,
    pub fields: Vec<(String, Value)>,
}

///
/// Value
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
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
