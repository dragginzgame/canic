//! Bounded child management engine. Ordinary approvals address two child slots.
//! The placement engine tracks shared shards, dedicated instances, workers and per-parent children under
//! one controller-approved subnet ceiling. Canisters own endpoints and lifecycle.
//! Host consumers use canic-management-protocol without this runtime.

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

#[cfg(feature = "auth")]
pub use engine::{auth_grant_issued, record_auth_grant};
pub use engine::{bootstrap, configure_snapshot_readers, custody, open};
#[cfg(feature = "fault-injection")]
pub use single::inject_fault;

pub use single::{observe_upgrade, status, upgrade_status};

/// Root placement grants, child allocation and standby pool management.
#[cfg(feature = "sharding")]
pub use engine::sharding;

/// Explicit child-scoped operations. The wrapper decides which endpoints to mount.

pub mod children {
    #[cfg(feature = "fault-injection")]
    pub use crate::engine::inject_fault;
    pub use crate::engine::{
        owner_custody as custody, status,
        upgrade::{observe as observe_upgrade, status as upgrade_status},
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
    pub use crate::engine::stored_api::custody;
}

/// Exact application lifecycle bindings, mounted explicitly by Root.

pub mod application {
    pub use crate::engine::application::{
        child_evidence, custody, custody_page, reconcile_child_creation, reconcile_child_upgrade,
        stage_child_creation, stage_child_upgrade,
    };
    #[cfg(feature = "blob")]
    pub use crate::engine::blob::activate;
    pub use crate::engine::directory::{list_children, redeliver_directory_revision};
}

/// Root-owned Store maintenance custody, mounted explicitly by Root.

pub mod store_maintenance {
    pub use crate::engine::store_maintenance::{maintain, observe, observe_attempt, recover};
}
