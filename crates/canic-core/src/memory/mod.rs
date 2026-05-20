//! Canic-managed stable-memory runtime boundary.
//!
//! This module is the Canic-owned adapter around `ic-memory` bootstrap and
//! Canic-specific stable-memory policy.

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
