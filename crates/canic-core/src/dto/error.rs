use crate::dto::prelude::*;

///
/// Error
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}

impl Error {
    #[inline]
    pub const fn new(code: ErrorCode, message: String) -> Self {
        Self { code, message }
    }

    /// 409
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Conflict, message.into())
    }

    /// 500-class failures
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, message.into())
    }

    /// 400 class failures
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message.into())
    }

    /// Broken invariant or impossible state
    pub fn invariant(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvariantViolation, message.into())
    }

    /// Resource / quota / capacity exhaustion
    pub fn exhausted(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ResourceExhausted, message.into())
    }

    /// 404
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message.into())
    }

    /// 401 / 403 class failures
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unauthorized, message.into())
    }
}

///
/// ErrorCode
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
#[remain::sorted]
pub enum ErrorCode {
    Conflict,
    Internal,
    InvalidInput,
    InvariantViolation,
    NotFound,
    ResourceExhausted,
    Unauthorized,
}
