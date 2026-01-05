use crate::{
    dto::memory::MemoryRegistryView, ops::runtime::memory::MemoryRegistryOps,
    workflow::memory::mapper::MemoryMapper,
};

///
/// MemoryQuery
///

pub struct MemoryQuery;

impl MemoryQuery {
    #[must_use]
    pub fn registry_view() -> MemoryRegistryView {
        let entries = MemoryRegistryOps::snapshot_entries();
        MemoryMapper::snapshot_entries_to_view(entries)
    }
}
