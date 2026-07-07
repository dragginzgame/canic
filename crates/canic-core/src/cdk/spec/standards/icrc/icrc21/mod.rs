//! Module: cdk::spec::standards::icrc::icrc21
//!
//! Responsibility: ICRC-21 Canister Call Consent Message Candid DTOs.
//! Does not own: consent policy, wallet UX, or application authorization.
//! Boundary: mirrors the external ICRC-21 surface for Canic callers.

//
// Keep these DTOs local rather than importing icrc-ledger-types. That crate
// pulls in broad dependencies and an older ic-stable-structures version.
//
mod errors;

pub use errors::*;

use crate::cdk::spec::prelude::*;

//
// ConsentInfo
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ConsentInfo {
    pub consent_message: ConsentMessage,
    pub metadata: ConsentMessageMetadata,
}

//
// ConsentMessage
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum ConsentMessage {
    GenericDisplayMessage(String),
    FieldsDisplayMessage(FieldsDisplay),
}

//
// ConsentMessageMetadata
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ConsentMessageMetadata {
    pub language: String,
    pub utc_offset_minutes: Option<i16>,
}

//
// ConsentMessageRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ConsentMessageRequest {
    pub method: String,
    pub arg: Vec<u8>,
    pub user_preferences: ConsentMessageSpec,
}

//
// ConsentMessageResponse
// Wrapper capturing the ok/error variants from an ICRC-21 consent request.
//

#[derive(CandidType, Deserialize)]
pub enum ConsentMessageResponse {
    Ok(ConsentInfo),
    Err(Icrc21Error),
}

impl From<Result<ConsentInfo, Icrc21Error>> for ConsentMessageResponse {
    fn from(res: Result<ConsentInfo, Icrc21Error>) -> Self {
        match res {
            Ok(info) => Self::Ok(info),
            Err(err) => Self::Err(err),
        }
    }
}

//
// ConsentMessageSpec
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ConsentMessageSpec {
    pub metadata: ConsentMessageMetadata,
    pub device_spec: Option<DisplayMessageType>,
}

//
// DisplayMessageType
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum DisplayMessageType {
    GenericDisplay,
    FieldsDisplay,
}

//
// FieldsDisplay
//

#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct FieldsDisplay {
    pub intent: String,
    pub fields: Vec<(String, Value)>,
}

//
// Value
//

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
