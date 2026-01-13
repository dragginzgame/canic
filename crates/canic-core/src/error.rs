use crate::access::AccessError;
use derive_more::Display;
use thiserror::Error as ThisError;

///
/// InternalError
///
/// Internal, structured error type.
///
/// This error:
/// - is NOT Candid-exposed
/// - is NOT stable across versions
/// - may evolve freely
///
/// All canister endpoints must convert this into a public error envelope
/// defined in dto/.
///

#[derive(Debug, ThisError)]
#[error("{message}")]
pub(crate) struct InternalError {
    class: InternalErrorClass,
    origin: InternalErrorOrigin,
    message: String,
}

impl InternalError {
    pub fn new(
        class: InternalErrorClass,
        origin: InternalErrorOrigin,
        message: impl Into<String>,
    ) -> Self {
        Self {
            class,
            origin,
            message: message.into(),
        }
    }

    pub fn domain(origin: InternalErrorOrigin, message: impl Into<String>) -> Self {
        Self::new(InternalErrorClass::Domain, origin, message)
    }

    pub fn invariant(origin: InternalErrorOrigin, message: impl Into<String>) -> Self {
        Self::new(InternalErrorClass::Invariant, origin, message)
    }

    pub fn infra(origin: InternalErrorOrigin, message: impl Into<String>) -> Self {
        Self::new(InternalErrorClass::Infra, origin, message)
    }

    pub fn ops(origin: InternalErrorOrigin, message: impl Into<String>) -> Self {
        Self::new(InternalErrorClass::Ops, origin, message)
    }

    pub fn workflow(origin: InternalErrorOrigin, message: impl Into<String>) -> Self {
        Self::new(InternalErrorClass::Workflow, origin, message)
    }

    pub const fn class(&self) -> InternalErrorClass {
        self.class
    }

    pub const fn origin(&self) -> InternalErrorOrigin {
        self.origin
    }

    #[must_use]
    pub const fn log_fields(&self) -> (InternalErrorClass, InternalErrorOrigin) {
        (self.class, self.origin)
    }
}

impl From<AccessError> for InternalError {
    fn from(err: AccessError) -> Self {
        Self::new(
            InternalErrorClass::Access,
            InternalErrorOrigin::Access,
            err.to_string(),
        )
    }
}

///
/// InternalErrorClass
///

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq)]
pub(crate) enum InternalErrorClass {
    Access,
    Domain,
    Infra,
    Ops,
    Workflow,
    Invariant,
}

///
/// InternalErrorOrigin
///

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq)]
pub(crate) enum InternalErrorOrigin {
    Access,
    Config,
    Domain,
    Infra,
    Ops,
    Storage,
    Workflow,
}
