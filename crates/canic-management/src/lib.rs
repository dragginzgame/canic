//! Bounded two-child management engine. Canisters own endpoints and lifecycle composition.
//! Host consumers use canic-management-protocol without this runtime.

#[cfg(target_arch = "wasm32")]
pub mod endpoint;
#[cfg(target_arch = "wasm32")]
mod engine;
#[cfg(target_arch = "wasm32")]
pub mod publication;
#[cfg(target_arch = "wasm32")]
mod single;
#[cfg(any(target_arch = "wasm32", test))]
mod storage;
#[cfg(target_arch = "wasm32")]
pub use engine::{custody, metrics::snapshot as metrics, open};
#[cfg(all(target_arch = "wasm32", feature = "fault-injection"))]
pub use single::inject_fault;
#[cfg(target_arch = "wasm32")]
pub use single::{
    observe_upgrade, reconcile, reconcile_upgrade, stage, stage_upgrade, status, upgrade_status,
};

/// Explicit child-scoped operations. The wrapper decides which endpoints to mount.
#[cfg(target_arch = "wasm32")]
pub mod children {
    #[cfg(feature = "fault-injection")]
    pub use crate::engine::inject_fault;
    pub use crate::engine::upgrade::{
        observe as observe_upgrade, reconcile as reconcile_upgrade, stage as stage_upgrade,
    };
    pub use crate::engine::{owner_custody as custody, reconcile, stage, status};
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
#[cfg(target_arch = "wasm32")]
pub mod stored {
    pub use crate::engine::stored_api::{
        custody, reconcile, reconcile_upgrade, root_custody, root_history, stage, stage_upgrade,
    };
}

/// Exact application lifecycle bindings, mounted explicitly by Root.
#[cfg(target_arch = "wasm32")]
pub mod application {
    pub use crate::engine::application::{custody, reconcile, stage};
    #[cfg(feature = "blob")]
    pub use crate::engine::application::{reconcile_upgrade, stage_upgrade};
    #[cfg(feature = "blob")]
    pub use crate::engine::blob::activate;
}

/// Root-owned Store maintenance custody, mounted explicitly by Root.
#[cfg(target_arch = "wasm32")]
pub mod store_maintenance {
    pub use crate::engine::store_maintenance::{
        activate, advance, install, observe, observe_attempt, recover, stage,
    };
}
