//! Module: dto::icrc21::errors
//!
//! Responsibility: ICRC-21 consent error DTOs.
//! Does not own: consent evaluation or wallet display behavior.
//! Boundary: mirrors the external ICRC-21 error surface.

//
// ErrorInfo
//

use candid::{CandidType, Deserialize, Nat};
use serde::Serialize;

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ErrorInfo {
    pub description: String,
}

//
// Icrc21Error
//

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
