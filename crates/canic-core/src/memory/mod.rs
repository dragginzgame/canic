//! Module: memory
//!
//! Responsibility: adapt Canic stable-memory declarations to `ic-memory` bootstrap.
//! Does not own: stable data schemas, ops storage APIs, or lifecycle orchestration.
//! Boundary: lifecycle initializes this before stable structures are accessed.

pub(crate) mod ledger;
mod manager;
mod policy;
pub mod registry;
pub mod runtime;

pub use crate::{eager_init, eager_static, ic_memory_key, ic_memory_range};

pub(crate) fn bootstrap_default_memory_manager() -> Result<
    ic_memory::ValidatedAllocations,
    ic_memory::RuntimeBootstrapError<registry::MemoryRegistryError>,
> {
    ic_memory::bootstrap_default_memory_manager_with_policy(&policy::CanicMemoryManagerPolicy::new())
}
