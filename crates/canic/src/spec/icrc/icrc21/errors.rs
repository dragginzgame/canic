use candid::{CandidType, Deserialize, Nat};
use serde::Serialize;

///
/// ErrorInfo
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ErrorInfo {
    pub description: String,
}

///
/// Icrc21Error
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Icrc21Error {
    UnsupportedCanisterCall(ErrorInfo),
    ConsentMessageUnavailable(ErrorInfo),
    InsufficientPayment(ErrorInfo),
    GenericError {
        error_code: Nat,
        description: String,
    },
}
