//! Canic-managed stable-memory runtime boundary.
//!
//! This module is the remaining Canic-owned adapter around the temporary
//! `canic-memory` crate while durable allocation-governance mechanics move to
//! `ic-memory`.

use crate::cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory};
use ::canic_memory::manager;
pub use ::canic_memory::{api, registry, runtime};

pub use crate::{eager_init, eager_static, ic_memory_key, ic_memory_range};

///
/// open_validated_memory
///

#[doc(hidden)]
#[must_use]
pub fn open_validated_memory(label: &str, id: u8) -> VirtualMemory<DefaultMemoryImpl> {
    runtime::assert_memory_bootstrap_ready(label, id);
    manager::MEMORY_MANAGER
        .with_borrow_mut(|mgr| mgr.get(crate::cdk::structures::memory::MemoryId::new(id)))
}
