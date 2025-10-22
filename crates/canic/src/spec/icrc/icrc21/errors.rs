use candid::{CandidType, Deserialize, Nat};
use serde::Serialize;

///
/// ErrorInfo
///

#[derive(Debug, CandidType, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorInfo {
    pub description: String,
}

///
/// Icrc21Error
///

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
