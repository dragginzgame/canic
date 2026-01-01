use crate::dto::prelude::*;
use std::fmt::{self, Display};

///
/// Error
///
/// Public-facing error DTO returned across the canister API boundary.
/// Encodes a stable error code and a human-readable message.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl Error {
    #[must_use]
    pub const fn new(code: ErrorCode, message: String) -> Self {
        Self { code, message }
    }

    /// 409 – Conflict with existing state or resource.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Conflict, message.into())
    }

    /// 403 – Authenticated caller is not permitted to perform this action.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Forbidden, message.into())
    }

    /// 500 – Internal or unexpected failure.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, message.into())
    }

    /// 400 – Invalid input or malformed request.
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message.into())
    }

    /// 500 – Broken invariant or impossible internal state.
    pub fn invariant(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvariantViolation, message.into())
    }

    /// 429 / 507 – Resource, quota, or capacity exhaustion.
    pub fn exhausted(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ResourceExhausted, message.into())
    }

    /// 404 – Requested resource was not found.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message.into())
    }

    /// 401 – Caller is unauthenticated or has an invalid identity.
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unauthorized, message.into())
    }
}

///
/// ErrorCode
///
/// Stable public error codes returned by the API.
/// New variants may be added in the future; consumers must handle unknown values.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
#[remain::sorted]
pub enum ErrorCode {
    Conflict,
    Forbidden,
    Internal,
    InvalidInput,
    InvariantViolation,
    NotFound,
    ResourceExhausted,
    Unauthorized,
}
