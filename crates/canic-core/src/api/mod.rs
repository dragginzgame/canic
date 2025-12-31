//! Public API façade for canister endpoints.
//!
//! This module contains thin wrappers exposed to proc-macro–generated
//! endpoints. Functions here translate public API calls into internal
//! workflow or ops calls and map internal errors into `PublicError`.
//!
//! No orchestration or business logic should live here.

pub mod endpoints;
pub mod timer;

///
/// EndpointCall
///

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EndpointCall {
    pub endpoint: EndpointId,
    pub kind: EndpointCallKind,
}

///
/// EndpointCallKind
///

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EndpointCallKind {
    Query,
    QueryComposite,
    Update,
}

///
/// EndpointId
///

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EndpointId {
    pub name: &'static str,
}

impl EndpointId {
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }
}
