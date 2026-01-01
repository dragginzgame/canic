//! Read-only projections for memory registry queries.

use crate::{
    dto::memory::{MemoryRegistryEntryView, MemoryRegistryView},
    ops::runtime::memory::MemoryOps,
};

///
/// Views
///

pub fn memory_registry_view() -> MemoryRegistryView {
    let entries = MemoryOps::snapshot_entries()
        .into_iter()
        .map(|entry| MemoryRegistryEntryView {
            id: entry.id,
            crate_name: entry.crate_name,
            label: entry.label,
        })
        .collect();

    MemoryRegistryView { entries }
}
