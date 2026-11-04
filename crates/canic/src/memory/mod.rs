//! Compose framework and application memory at the single lifecycle owner.
pub use canic_core::memory::*;

/// Complete requests for the selected framework role and features.
pub fn requests() -> Result<
    Vec<canic_core::__reexports::ic_storage::MemoryRequest>,
    canic_core::__reexports::ic_storage::AdmissionError,
> {
    let requests = canic_core::memory::requests()?;
    #[cfg(any(
        feature = "control-plane",
        feature = "fleet-coordinator-canister",
        feature = "wasm-store-canister"
    ))]
    let requests = requests
        .into_iter()
        .chain(canic_control_plane::memory::requests()?)
        .collect();
    Ok(requests)
}

/// Framework-only host composition. Applications extend its requests and grants.
///
/// # Panics
/// Panics if authored framework requests or grants are invalid.
#[must_use]
pub fn framework() -> MemoryBootstrap {
    MemoryBootstrap {
        requests: requests().expect("framework memory requests"),
        grants: grants::framework_grants(Vec::new()).expect("framework memory grants"),
        admission: None,
    }
}
