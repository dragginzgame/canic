use crate::dto::prelude::*;

///
/// Error
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}

///
/// ErrorCode
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum ErrorCode {
    Unauthorized,
    InvalidInput,
    NotFound,
    Conflict,
    InvariantViolation,
    ResourceExhausted,
    Internal,
}
