//! Read-only projections for memory registry queries.

use crate::{
    dto::memory::MemoryRegistryView, ops::runtime::memory::MemoryOps,
    workflow::memory::mapper::MemoryRegistryMapper,
};

///
/// Views
///

#[must_use]
pub fn memory_registry_view() -> MemoryRegistryView {
    let entries = MemoryOps::snapshot_entries();
    MemoryRegistryMapper::snapshot_entries_to_view(entries)
}
