use crate::{
    InternalError,
    access::AccessError,
    dto::error::{Error, ErrorCode},
};

///
/// InternalError
///

impl InternalError {
    pub fn public(&self) -> Error {
        match self {
            // ---------------------------------------------------------
            // Access / authorization
            // ---------------------------------------------------------
            Self::Access(err) => access_error(err),

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

    fn public_message(code: ErrorCode, message: &'static str) -> Error {
        Error {
            code,
            message: message.to_string(),
        }
    }
}

fn access_error(err: &AccessError) -> Error {
    match err {
        AccessError::Denied(reason) => Error::unauthorized(reason.clone()),
        _ => Error::unauthorized("unauthorized"),
    }
}

impl From<&InternalError> for Error {
    fn from(err: &InternalError) -> Self {
        err.public()
    }
}

impl From<InternalError> for Error {
    fn from(err: InternalError) -> Self {
        Self::from(&err)
    }
}

impl From<AccessError> for Error {
    fn from(err: AccessError) -> Self {
        match err {
            AccessError::Auth(e) => Self::new(ErrorCode::Unauthorized, e.to_string()),
            AccessError::Denied(reason) => Self::new(ErrorCode::Forbidden, reason),
            AccessError::Env(e) => Self::new(ErrorCode::Forbidden, e.to_string()),
            AccessError::Guard(e) => Self::new(ErrorCode::Forbidden, e.to_string()),
            AccessError::Rule(e) => Self::new(ErrorCode::Forbidden, e.to_string()),
        }
    }
}
