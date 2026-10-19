//! Module: dto::icrc21::errors
//!
//! Responsibility: ICRC-21 consent error DTOs.
//! Does not own: consent evaluation or wallet display behavior.
//! Boundary: mirrors the external ICRC-21 error surface.

//
// ErrorInfo
//

use ic_platform::candid::contract::{CandidType, Nat};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct ErrorInfo {
    pub description: String,
}

//
// Icrc21Error
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum Icrc21Error {
    UnsupportedCanisterCall(ErrorInfo),
    ConsentMessageUnavailable(ErrorInfo),
    InsufficientPayment(ErrorInfo),
    GenericError {
        error_code: Nat,
        description: String,
    },
}
