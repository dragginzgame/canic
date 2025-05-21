use crate::memory::{
    MEMORY_REGISTRY,
    registry::{Registry, RegistryData, RegistryEntry},
};

/// Get all registered memory IDs and their entries
#[must_use]
pub fn get_data() -> RegistryData {
    MEMORY_REGISTRY.with_borrow(Registry::get_data)
}

/// Get the next available memory ID
///
/// This function finds the smallest unused ID starting from 1,
/// as 0 is reserved for the registry itself.
#[must_use]
pub fn next_available_id() -> u8 {
    MEMORY_REGISTRY.with_borrow(|registry| registry.next_available_id())
}

/// Register a memory area with a specific ID
///
/// Returns the ID that was used for registration
pub fn register(id: u8, entry: RegistryEntry) -> Result<u8, crate::Error> {
    MEMORY_REGISTRY.with_borrow_mut(|registry| {
        registry.register(id, entry)
            .map_err(|e| crate::memory::MemoryError::RegistryError(e).into())
    })
}

/// Register a memory area with an auto-assigned ID
///
/// This function automatically finds the next available ID and uses it
/// for registration. It returns the assigned ID on success.
pub fn register_auto(entry: RegistryEntry) -> Result<u8, crate::Error> {
    MEMORY_REGISTRY.with_borrow_mut(|registry| {
        registry.register_auto(entry)
            .map_err(|e| crate::memory::MemoryError::RegistryError(e).into())
    })
}
