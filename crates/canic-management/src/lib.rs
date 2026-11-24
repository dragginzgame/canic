//! Bounded child management engine. Ordinary approvals address two child slots.
//! The optional placement engine retains separate shard and project identities under
//! one controller-approved subnet ceiling. Canisters own endpoints and lifecycle.
//! Host consumers use canic-management-protocol without this runtime.

#[cfg(feature = "backup")]
pub mod backup;

pub mod endpoint;

pub mod funding;

mod engine;
#[cfg(feature = "role-issuer")]
pub use engine::role_issuer;
mod mount;

pub mod publication;

mod single;
mod storage;

#[cfg(feature = "sharding")]
mod placement;

pub use engine::{bootstrap, custody, open};
#[cfg(feature = "fault-injection")]
pub use single::inject_fault;

pub use single::{
    observe_upgrade, reconcile, reconcile_upgrade, stage, stage_upgrade, status, upgrade_status,
};

/// Operator approval and authenticated hub operations for one Root-owned shard pool.
#[cfg(feature = "sharding")]
pub use engine::sharding;

/// Explicit child-scoped operations. The wrapper decides which endpoints to mount.

pub mod children {
    #[cfg(feature = "fault-injection")]
    pub use crate::engine::inject_fault;
    pub use crate::engine::{
        owner_custody as custody, reconcile, stage, status,
        upgrade::{
            observe as observe_upgrade, reconcile as reconcile_upgrade, stage as stage_upgrade,
            status as upgrade_status,
        },
    };
}

/// Application-owned bounded readiness request and response admission.
/// The engine observes the exact installed module and controller before calling this method.
#[derive(Clone)]
pub struct Readiness {
    /// Replicated application method, invoked with a five-second bounded wait.
    pub method: &'static str,
    /// Exact encoded request supplied by the application composition root.
    pub request: Vec<u8>,
    /// Bounded response decoder and application readiness check.
    pub accept: fn(&[u8]) -> Result<(), canic_management_protocol::Failure>,
}

/// Store-bound operations, mounted explicitly by the owning canister.

pub mod stored {
    pub use crate::engine::stored_api::{
        custody, reconcile, reconcile_upgrade, root_custody, root_history, stage, stage_upgrade,
    };
}

/// Exact application lifecycle bindings, mounted explicitly by Root.

pub mod application {
    pub use crate::engine::application::{child_evidence, custody, reconcile, stage};
    #[cfg(feature = "blob")]
    pub use crate::engine::application::{reconcile_upgrade, stage_upgrade};
    #[cfg(feature = "blob")]
    pub use crate::engine::blob::activate;
    pub use crate::engine::directory::publish_directory;
}

/// Root-owned Store maintenance custody, mounted explicitly by Root.

pub mod store_maintenance {
    pub use crate::engine::store_maintenance::{
        activate, advance, install, observe, observe_attempt, recover, stage,
    };
}
