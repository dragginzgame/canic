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
pub mod bootstrap;
pub mod cascade;
pub mod config;
pub mod error;
pub mod ic;
pub mod icts;
pub mod lifecycle;
pub mod placement;
pub mod pool;
pub mod rpc;
pub mod state;
pub mod timer;
pub mod topology;
pub mod wasm;

///
/// Query Wrappers
/// (these modules have nothing else other than safe, public Query APIs)
///

pub mod cycles {
    pub use crate::workflow::runtime::cycles::query::CycleTrackerQuery;
}
pub mod env {
    pub use crate::workflow::env::query::EnvQuery;
}
pub mod icrc {
    pub use crate::workflow::icrc::query::{Icrc10Query, Icrc21Query};
}
pub mod log {
    pub use crate::workflow::log::query::LogQuery;
}
pub mod memory {
    pub use crate::workflow::memory::query::MemoryQuery;
}
pub mod metrics {
    pub use crate::workflow::metrics::query::MetricsQuery;
}

///
/// Prelude
///

pub mod prelude {
    pub use crate::{
        PublicError,
        cdk::types::{Account, Principal},
    };
}

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
