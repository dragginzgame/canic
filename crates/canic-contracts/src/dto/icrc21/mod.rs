//! Module: dto::icrc21
//!
//! Responsibility: ICRC-21 Canister Call Consent Message Candid DTOs.
//! Does not own: consent policy, wallet UX, or application authorization.
//! Boundary: mirrors the external ICRC-21 surface for Canic callers.

// Required external contract: selected Platform 0.6.19 does not yet export
// the ICRC-21 types from its pinned icrc-ledger-types 0.2.0 dependency.
// Replace these mirrors when that public supplier surface is available.

mod errors;

use crate::dto::prelude::*;
use ic_platform::candid::contract::CandidType;

pub use errors::*;

//
// ConsentInfo
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct ConsentInfo {
    pub consent_message: ConsentMessage,
    pub metadata: ConsentMessageMetadata,
}

//
// ConsentMessage
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub enum ConsentMessage {
    GenericDisplayMessage(String),
    FieldsDisplayMessage(FieldsDisplay),
}

//
// ConsentMessageMetadata
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct ConsentMessageMetadata {
    pub language: String,
    pub utc_offset_minutes: Option<i16>,
}

//
// ConsentMessageRequest
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct ConsentMessageRequest {
    pub method: String,
    pub arg: Vec<u8>,
    pub user_preferences: ConsentMessageSpec,
}

//
// ConsentMessageSpec
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct ConsentMessageSpec {
    pub metadata: ConsentMessageMetadata,
    pub device_spec: Option<DisplayMessageType>,
}

//
// DisplayMessageType
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub enum DisplayMessageType {
    GenericDisplay,
    FieldsDisplay,
}

//
// FieldsDisplay
//

#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct FieldsDisplay {
    pub intent: String,
    pub fields: Vec<(String, Value)>,
}

//
// Value
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
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
