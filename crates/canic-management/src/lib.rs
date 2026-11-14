//! Bounded one-child management engine. Canisters own endpoints and lifecycle composition.
//! Host consumers use canic-management-protocol without this runtime.

#[cfg(target_arch = "wasm32")]
pub mod endpoint;
#[cfg(target_arch = "wasm32")]
mod engine;
#[cfg(all(target_arch = "wasm32", feature = "fault-injection"))]
pub use engine::inject_fault;
#[cfg(target_arch = "wasm32")]
pub use engine::{open, reconcile, stage, status};

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
