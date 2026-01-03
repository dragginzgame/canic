//! Public API façade for canister endpoints.
//!
//! This module contains thin wrappers exposed to proc-macro–generated
//! endpoints. Functions here translate public API calls into internal
//! workflow or ops calls and map internal errors into `PublicError`.
//!
//! No orchestration or business logic should live here.
//! Any wrapper callable from an endpoint must return a `Result` so errors
//! are consistently mapped at the boundary.

pub mod access;
pub mod app;
pub mod cascade;
pub mod config;
pub mod cycles;
pub mod env;
pub mod error;
pub mod ic;
pub mod icrc;
pub mod icts;
pub mod instrumentation;
pub mod lifecycle;
pub mod log;
pub mod memory;
pub mod metrics;
pub mod placement;
pub mod pool;
pub mod rpc;
pub mod state;
pub mod timer;
pub mod topology;
pub mod wasm;

///
/// EndpointCall
///

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EndpointCall {
    pub endpoint: EndpointId,
    pub kind: EndpointCallKind,
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

///
/// EndpointCallKind
///

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EndpointCallKind {
    Query,
    QueryComposite,
    Update,
}
