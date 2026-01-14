use crate::{dto::memory::MemoryRegistryView, ops::runtime::memory::MemoryRegistryOps};

///
/// MemoryQuery
///

pub struct MemoryQuery;

impl MemoryQuery {
    #[must_use]
    pub fn registry_view() -> MemoryRegistryView {
        let entries = MemoryRegistryOps::snapshot_entries();
        MemoryRegistryView { entries }
    }
}
