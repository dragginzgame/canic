//! Explicit requests for the selected control-plane roles.
pub(crate) mod keys;
use canic_core::__reexports::ic_memory;
#[cfg(any(
    feature = "fleet-coordinator-canister",
    feature = "root-control-plane",
    feature = "wasm-store-canister"
))]
const MEMORY_AUTHORITY: &str = canic_core::memory::CANIC_CONTROL_PLANE_MEMORY_AUTHORITY;
/// Complete selected control-plane store requests.
pub fn requests() -> Result<Vec<ic_memory::MemoryRequest>, ic_memory::MemoryDeclarationError> {
    keys::requests()
}

/// Host composition for the selected control-plane roles.
///
/// # Panics
/// Panics if authored framework requests or grants are invalid.
#[must_use]
pub fn framework() -> canic_core::memory::MemoryBootstrap {
    canic_core::memory::MemoryBootstrap {
        requests: canic_core::memory::requests()
            .expect("Core requests")
            .into_iter()
            .chain(requests().expect("Control Plane requests"))
            .collect(),
        pool: canic_core::memory::pool::framework_pool(Vec::new(), Vec::new())
            .expect("framework grants"),
        admission: None,
    }
}
