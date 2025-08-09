use candid::CandidType;
use serde::Deserialize;

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

#[derive(CandidType, Deserialize)]
pub struct Icrc21ConsentInfo {
    pub consent_message: Icrc21ConsentMessage,
    pub metadata: Icrc21ConsentMessageMetadata,
}

#[derive(CandidType, Clone, Deserialize)]
pub struct Icrc21ConsentMessageSpec {
    pub metadata: Icrc21ConsentMessageMetadata,
    pub device_spec: Option<Icrc21DeviceSpec>,
}

#[derive(CandidType, Deserialize)]
pub struct Icrc21ConsentMessageRequest {
    pub method: String,
    pub arg: Vec<u8>,
    pub user_preferences: Icrc21ConsentMessageSpec,
}

#[derive(CandidType, Deserialize)]
pub enum Icrc21ConsentMessageResponse {
    Ok(Icrc21ConsentInfo),
    Err(Icrc21Error),
}

#[derive(CandidType, Clone, Deserialize)]
pub struct Icrc21ConsentMessageMetadata {
    pub language: String,
    pub utc_offset_minutes: Option<i16>,
}

///
/// Icrc21ConsentPreferences
///

#[derive(CandidType, Clone, Deserialize)]
pub struct Icrc21ConsentPreferences {
    pub language: String,
}

#[derive(CandidType, Clone, Deserialize)]
pub enum Icrc21DeviceSpec {
    GenericDisplay,
    FieldsDisplay,
}

///
/// Icrc21 Values
///

#[derive(CandidType, Deserialize)]
pub enum Icrc21Value {
    DurationSeconds(Icrc21DurationSeconds),
    Text(Icrc21TextValue),
    TimestampSeconds(Icrc21TimestampSeconds),
    TokenAmount(Icrc21TokenAmount),
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
pub enum Icrc21Error {
    ConsentMessageUnavailable(Icrc21ErrorInfo),
    GenericError {
        error_code: u64,
        description: String,
    },
    InsufficientPayment(Icrc21ErrorInfo),
    UnsupportedCanisterCall(Icrc21ErrorInfo),
}

#[derive(CandidType, Deserialize)]
pub struct Icrc21ErrorInfo {
    pub description: String,
}
