pub use crate::model::memory::registry::{MemoryRange, MemoryRegistryEntry};

///
/// MemoryRegistry
/// Ops wrapper around the global memory registry.
///

pub struct MemoryRegistry;

impl MemoryRegistry {
    #[must_use]
    pub fn export() -> Vec<(u8, MemoryRegistryEntry)> {
        crate::model::memory::registry::MemoryRegistry::export()
    }

    #[must_use]
    pub fn export_ranges() -> Vec<(String, MemoryRange)> {
        crate::model::memory::registry::MemoryRegistry::export_ranges()
    }

    #[must_use]
    pub fn get(id: u8) -> Option<MemoryRegistryEntry> {
        crate::model::memory::registry::MemoryRegistry::get(id)
    }
}
