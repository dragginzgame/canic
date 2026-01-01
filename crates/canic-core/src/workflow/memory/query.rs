//! Read-only projections for memory registry queries.

use crate::workflow::memory::mapper::MemoryRegistryMapper;
use crate::{dto::memory::MemoryRegistryView, ops::runtime::memory::MemoryOps};

///
/// Views
///

pub fn memory_registry_view() -> MemoryRegistryView {
    let entries = MemoryOps::snapshot_entries();
    MemoryRegistryMapper::snapshot_entries_to_view(entries)
}
