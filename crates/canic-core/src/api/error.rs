use crate::{InternalError, InternalErrorClass, InternalErrorOrigin, dto::error::Error};

fn internal_error_to_public(err: &InternalError) -> Error {
    match err.class() {
        InternalErrorClass::Access => Error::unauthorized(err.to_string()),

        InternalErrorClass::Domain => match err.origin() {
            InternalErrorOrigin::Config => Error::invalid("invalid configuration"),
            _ => Error::conflict("policy rejected"),
        },

        InternalErrorClass::Invariant => Error::invariant("invariant violation"),

        InternalErrorClass::Infra | InternalErrorClass::Ops | InternalErrorClass::Workflow => {
            Error::internal("internal error")
        }
    }
}

impl From<&InternalError> for Error {
    fn from(err: &InternalError) -> Self {
        internal_error_to_public(err)
    }
}

impl From<InternalError> for Error {
    fn from(err: InternalError) -> Self {
        internal_error_to_public(&err)
    }
}
