use crate::{dto::memory::MemoryRegistryResponse, ops::runtime::memory::MemoryRegistryOps};

///
/// MemoryQuery
///

pub struct MemoryQuery;

impl MemoryQuery {
    #[must_use]
    pub fn registry() -> MemoryRegistryResponse {
        let entries = MemoryRegistryOps::snapshot_entries();
        MemoryRegistryResponse { entries }
    }
}
