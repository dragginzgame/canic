//! Module: memory
//!
//! Responsibility: adapt Canic stable-memory declarations to `ic-memory` bootstrap.
//! Also owns physical stable-memory extent observation for runtime metrics.
//! Does not own: stable data schemas, ops storage APIs, or lifecycle orchestration.
//! Boundary: lifecycle initializes this before stable structures are accessed.

pub(crate) mod ledger;
mod policy;
pub mod registry;
pub mod runtime;

pub use crate::{eager_init, eager_static, ic_memory_key, ic_memory_range};

/// Stable allocation-policy authority for Canic core memory declarations.
pub const CANIC_CORE_MEMORY_AUTHORITY: &str = "canic-core";
/// Stable allocation-policy authority for Canic control-plane memory declarations.
pub const CANIC_CONTROL_PLANE_MEMORY_AUTHORITY: &str = "canic-control-plane";

/// Bucket size compiled into this artifact, in 64 KiB Wasm pages.
///
/// # Panics
/// Panics if the build script supplies an invalid value; normal builds validate it first.
#[must_use]
pub fn configured_bucket_pages() -> u16 {
    env!("CANIC_MEMORY_BUCKET_PAGES")
        .parse()
        .expect("build-validated memory bucket size")
}

/// Observe the current physical stable-memory extent without allocating or reading data.
#[cfg(target_arch = "wasm32")]
pub(crate) fn stable_extent_bytes() -> u64 {
    ic_cdk::api::stable_size().saturating_mul(65_536)
}

pub(crate) fn bootstrap_default_memory_manager()
-> Result<(), ic_memory::RuntimeBootstrapError<registry::MemoryRegistryError>> {
    let config = ic_memory::MemoryManagerConfig::new(configured_bucket_pages())
        .map_err(ic_memory::RuntimeStateError::from)?;
    ic_memory::bootstrap_default_memory_manager_with_config(
        config,
        &policy::CanicMemoryManagerPolicy::new(),
    )
    .map(|_| ())
}
