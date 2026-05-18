//! Canic-managed stable-memory runtime boundary.
//!
//! This module is the Canic-owned adapter around current stable-memory
//! bootstrap mechanics while durable allocation-governance primitives move
//! into `ic-memory`.

use crate::cdk::structures::{DefaultMemoryImpl, memory::VirtualMemory};

pub mod api;
mod ledger;
mod manager;
mod policy;
pub mod registry;
pub mod runtime;

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
