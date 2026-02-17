use crate::{InternalError, InternalErrorClass, InternalErrorOrigin, dto::error::Error};

fn internal_error_to_public(err: &InternalError) -> Error {
    if let Some(public) = err.public_error() {
        return public.clone();
    }

    match err.class() {
        InternalErrorClass::Access => Error::unauthorized(err.to_string()),

        InternalErrorClass::Domain => match err.origin() {
            InternalErrorOrigin::Config => Error::invalid(err.to_string()),
            _ => Error::conflict(err.to_string()),
        },

        InternalErrorClass::Invariant => Error::invariant(err.to_string()),

        InternalErrorClass::Infra | InternalErrorClass::Ops | InternalErrorClass::Workflow => {
            Error::internal(err.to_string())
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
