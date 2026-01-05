use crate::{
    dto::memory::{MemoryRegistryEntryView, MemoryRegistryView},
    ops::runtime::memory::MemoryRegistryEntrySnapshot,
};

///
/// MemoryMapper
///

pub struct MemoryMapper;

impl MemoryMapper {
    #[must_use]
    pub fn entry_snapshot_to_view(entry: MemoryRegistryEntrySnapshot) -> MemoryRegistryEntryView {
        MemoryRegistryEntryView {
            id: entry.id,
            crate_name: entry.crate_name,
            label: entry.label,
        }
    }

    #[must_use]
    pub fn snapshot_entries_to_view(
        entries: Vec<MemoryRegistryEntrySnapshot>,
    ) -> MemoryRegistryView {
        let entries = entries
            .into_iter()
            .map(Self::entry_snapshot_to_view)
            .collect();

        MemoryRegistryView { entries }
    }
}
