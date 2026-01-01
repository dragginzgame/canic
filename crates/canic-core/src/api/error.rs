use crate::{
    Error,
    dto::error::{Error as PublicError, ErrorCode},
};

impl Error {
    pub fn public(&self) -> PublicError {
        match self {
            // ---------------------------------------------------------
            // Access / authorization
            // ---------------------------------------------------------
            Self::Access(_) => Self::public_message(ErrorCode::Unauthorized, "unauthorized"),

            // ---------------------------------------------------------
            // Input / configuration
            // ---------------------------------------------------------
            Self::Config(_) => {
                Self::public_message(ErrorCode::InvalidInput, "invalid configuration")
            }

            // ---------------------------------------------------------
            // Policy decisions
            // ---------------------------------------------------------
            Self::Domain(_) => Self::public_message(ErrorCode::Conflict, "policy rejected"),

            // ---------------------------------------------------------
            // State / invariants
            // ---------------------------------------------------------
            Self::Storage(_) => {
                Self::public_message(ErrorCode::InvariantViolation, "invariant violation")
            }

            // ---------------------------------------------------------
            // Infrastructure / execution
            // ---------------------------------------------------------
            Self::Infra(_) | Self::Ops(_) | Self::Workflow(_) => {
                Self::public_message(ErrorCode::Internal, "internal error")
            }
        }
    }

    fn public_message(code: ErrorCode, message: &'static str) -> PublicError {
        PublicError {
            code,
            message: message.to_string(),
        }
    }
}

impl From<&Error> for PublicError {
    fn from(err: &Error) -> Self {
        err.public()
    }
}

impl From<Error> for PublicError {
    fn from(err: Error) -> Self {
        Self::from(&err)
    }
}
